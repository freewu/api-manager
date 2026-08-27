//! 由 import.rs 拆分：JMeter
use super::*;
#[allow(unused_imports)]
use crate::{ApiFile, BodyData, DocParam, EnvVariable, KeyValue, MockConfig, ResponseItem, sanitize_filename, unique_path, workspace_root, ENV_FILE, INFO_FILE};
#[allow(unused_imports)]
use serde::Serialize;
#[allow(unused_imports)]
use serde_json::Value;
#[allow(unused_imports)]
use std::collections::{BTreeMap, HashMap};
#[allow(unused_imports)]
use std::fs;
#[allow(unused_imports)]
use std::path::{Path, PathBuf};

pub(crate) fn import_jmeter_file(root: &Path, file: &Path) -> Result<OpenApiImportResult, String> {
    let content = fs::read_to_string(file).map_err(|e| format!("读取文件失败: {e}"))?;
    // JMX 中 URL 参数常带未转义 &，解析前清洗
    let content = sanitize_jmx_entities(&content);
    let doc = roxmltree::Document::parse(&content).map_err(|e| format!("解析 JMX 失败: {e}"))?;
    let root_el = doc.root_element();
    if root_el.tag_name().name() != "jmeterTestPlan" {
        return Err("不是有效的 JMeter 测试计划（缺少 jmeterTestPlan 根节点）".into());
    }
    // TestPlan 用户定义变量
    let mut vars: HashMap<String, String> = HashMap::new();
    for tp in root_el
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "TestPlan")
    {
        for arg in tp
            .descendants()
            .filter(|n| n.is_element() && n.attribute("elementType") == Some("Argument"))
        {
            let name = jmeter_child_string(arg, "Argument.name");
            let value = jmeter_child_string(arg, "Argument.value");
            if !name.is_empty() {
                vars.insert(name, value);
            }
        }
    }
    // TestPlan testname 作顶层分组名
    let plan_name = root_el
        .descendants()
        .find(|n| n.is_element() && n.tag_name().name() == "TestPlan")
        .and_then(|n| n.attribute("testname"))
        .unwrap_or("JMeter 导入")
        .to_string();
    let folder = unique_path(root, &plan_name, "");
    fs::create_dir_all(&folder).map_err(|e| format!("创建分组失败: {e}"))?;
    let src_name = file
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    // 用户变量中的 host 作为 base_url
    let host_var = vars.get("host").cloned().unwrap_or_default();
    let base_url = if host_var.is_empty() {
        None
    } else {
        Some(host_var.clone())
    };
    write_pretty(
        &folder.join(INFO_FILE),
        &InfoJson {
            name: Some(plan_name.clone()),
            description: format!("从 JMeter 测试计划导入（{src_name}）"),
            base_url,
            mock_port: None,
            order: None,
            collapsed: None,
            deprecated: None,
        },
    )?;
    let mut count = 0usize;
    let mut stats = ImportStats::default();
    // 递归处理所有 hashTree（TestPlan 级 sampler 罕见；ThreadGroup 为分组）
    let mut pending_headers: Vec<KeyValue> = Vec::new();
    let mut pending_group: Option<String> = None;
    count += jmeter_walk_hash_tree(
        &root_el,
        &folder,
        &vars,
        &mut pending_headers,
        &mut pending_group,
        &mut stats,
    )?;
    Ok(OpenApiImportResult {
        folder: folder.to_string_lossy().to_string(),
        count,
        http: stats.http,
        ws: stats.ws,
        graphql: stats.graphql,
        socketio: stats.socketio,
        ..Default::default()
    })
}

/// 把裸 & 转义为 &amp;（保留合法实体）
fn sanitize_jmx_entities(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while !rest.is_empty() {
        let ch = rest.chars().next().unwrap();
        if ch == '&' {
            let after = &rest[ch.len_utf8()..];
            let valid = after.starts_with("amp;")
                || after.starts_with("lt;")
                || after.starts_with("gt;")
                || after.starts_with("quot;")
                || after.starts_with("apos;")
                || after.starts_with('#');
            if valid {
                out.push('&');
            } else {
                out.push_str("&amp;");
            }
        } else {
            out.push(ch);
        }
        rest = &rest[ch.len_utf8()..];
    }
    out
}

/// 递归遍历元素树：HeaderManager 更新作用域 headers，HTTPSamplerProxy 写接口，ThreadGroup 建分组，hashTree 递归
fn jmeter_walk_hash_tree(
    el: &roxmltree::Node,
    dir: &Path,
    vars: &HashMap<String, String>,
    pending_headers: &mut Vec<KeyValue>,
    pending_group: &mut Option<String>,

    stats: &mut ImportStats) -> Result<usize, String> {
    let mut count = 0usize;
    for child in el.children().filter(|n| n.is_element()) {
        match child.tag_name().name() {
            "HeaderManager" => {
                let mut hs: Vec<KeyValue> = Vec::new();
                for h in child
                    .descendants()
                    .filter(|n| n.is_element() && n.attribute("elementType") == Some("Header"))
                {
                    let key = jmeter_child_string(h, "Header.name");
                    let value = jmeter_child_string(h, "Header.value");
                    if !key.is_empty() {
                        hs.push(KeyValue {
                            key,
                            value,
                            enabled: true,
                            is_file: false,
                            description: String::new(),
                        });
                    }
                }
                if !hs.is_empty() {
                    *pending_headers = hs;
                }
            }
            "ThreadGroup" => {
                if let Some(n) = child.attribute("testname") {
                    *pending_group = Some(n.to_string());
                }
            }
            "HTTPSamplerProxy" => {
                count += jmeter_sampler_to_api(child, dir, vars, pending_headers, stats)?;
            }
            "hashTree" => {
                if let Some(gname) = pending_group.take() {
                    let sub_base = sanitize_filename(&gname);
                    let sub_base = if sub_base.is_empty() {
                        "线程组".to_string()
                    } else {
                        sub_base
                    };
                    let sub_dir = dir.join(&sub_base);
                    if !sub_dir.is_dir() {
                        fs::create_dir_all(&sub_dir).map_err(|e| format!("创建分组失败: {e}"))?;
                        write_pretty(
                            &sub_dir.join(INFO_FILE),
                            &InfoJson {
                                name: Some(gname),
                                description: String::new(),
                                base_url: None,
                                mock_port: None,
                                order: None,
                                collapsed: None,
                                deprecated: None,
                            },
                        )?;
                    }
                    count += jmeter_walk_hash_tree(
                        &child,
                        &sub_dir,
                        vars,
                        pending_headers,
                        pending_group,
                        stats,
                    )?;
                } else {
                    count += jmeter_walk_hash_tree(
                        &child,
                        dir,
                        vars,
                        pending_headers,
                        pending_group,
                        stats,
                    )?;
                }
            }
            _ => {}
        }
    }
    Ok(count)
}

/// 读取元素下指定名称的 stringProp 子节点文本
fn jmeter_child_string(el: roxmltree::Node, prop: &str) -> String {
    el.children()
        .find(|n| n.is_element() && n.tag_name().name() == "stringProp" && n.attribute("name") == Some(prop))
        .map(|n| n.text().unwrap_or("").to_string())
        .unwrap_or_default()
}

/// JMeter sampler → ApiFile
fn jmeter_sampler_to_api(
    el: roxmltree::Node,
    dir: &Path,
    vars: &HashMap<String, String>,
    headers: &[KeyValue],

    stats: &mut ImportStats) -> Result<usize, String> {
    let name = el.attribute("testname").unwrap_or("未命名接口").to_string();
    let mut method = jmeter_child_string(el, "HTTPSampler.method");
    if method.is_empty() {
        method = "GET".to_string();
    }
    let method = method.to_uppercase();
    let mut domain = jmeter_child_string(el, "HTTPSampler.domain");
    let mut path = jmeter_child_string(el, "HTTPSampler.path");
    if path.is_empty() {
        return Ok(0);
    }
    // ${var} 替换
    for (k, v) in vars {
        domain = domain.replace(&format!("${{{k}}}"), v);
        path = path.replace(&format!("${{{k}}}"), v);
    }
    let protocol = jmeter_child_string(el, "HTTPSampler.protocol");
    let is_ws = domain.starts_with("ws://") || domain.starts_with("wss://");
    let api_protocol = if is_ws {
        "websocket".to_string()
    } else {
        "http".to_string()
    };
    let (clean_path, params) = if is_ws {
        (path.clone(), Vec::new())
    } else {
        extract_path(&path)
    };
    let mut query = Vec::new();
    // path 中的 ?a=b（extract_path 已剥离 query，从原始 path 提取）
    if let Some(qi) = path.find('?') {
        let qs = &path[qi + 1..];
        for pair in qs.split('&') {
            let mut it = pair.splitn(2, '=');
            let key = it.next().unwrap_or("").trim().to_string();
            if key.is_empty() {
                continue;
            }
            let value = it.next().unwrap_or("").to_string();
            query.push(KeyValue {
                key,
                value,
                enabled: true,
                is_file: false,
                description: String::new(),
            });
        }
    }
    // HTTPsampler.Arguments：body（POST）或 query 参数（GET）
    let mut body = BodyData::default();
    if let Some(args) = el
        .children()
        .find(|n| n.is_element() && n.attribute("name") == Some("HTTPsampler.Arguments"))
    {
        let mut http_args: Vec<(String, String, String)> = Vec::new();
        for a in args
            .descendants()
            .filter(|n| n.is_element() && n.attribute("elementType") == Some("HTTPArgument"))
        {
            let aname = a.attribute("name").unwrap_or("").to_string();
            let aval = jmeter_child_string(a, "Argument.value");
            let meta = jmeter_child_string(a, "Argument.metadata");
            http_args.push((aname, aval, meta));
        }
        let is_write = matches!(method.as_str(), "POST" | "PUT" | "PATCH" | "DELETE");
        if is_write && !http_args.is_empty() {
            // 单个无名参数且值为 JSON → json body
            let single = http_args.len() == 1 && http_args[0].0.is_empty();
            let first = http_args[0].1.trim();
            if single && (first.starts_with('{') || first.starts_with('[')) {
                if !first.is_empty() {
                    body.mode = "json".into();
                    body.raw = first.to_string();
                }
            } else if http_args.iter().all(|(n, _, _)| !n.is_empty()) {
                let mut form = Vec::new();
                for (n, v, _) in http_args {
                    if n.is_empty() {
                        continue;
                    }
                    form.push(KeyValue {
                        key: n,
                        value: v,
                        enabled: true,
                        is_file: false,
                        description: String::new(),
                    });
                }
                if !form.is_empty() {
                    body.mode = "form".into();
                    body.form = form;
                }
            }
        } else if !is_write {
            for (n, v, _) in http_args {
                if n.is_empty() {
                    continue;
                }
                query.push(KeyValue {
                    key: n,
                    value: v,
                    enabled: true,
                    is_file: false,
                    description: String::new(),
                });
            }
        }
    }
    let mut url = String::new();
    if is_ws {
        url = clean_path.clone();
    } else if !domain.is_empty() {
        if domain.contains("://") {
            url = format!("{domain}{clean_path}");
        } else if !protocol.is_empty() && !domain.is_empty() {
            url = format!("{protocol}://{domain}{clean_path}");
        } else {
            url = format!("https://{domain}{clean_path}");
        }
    }
    let api_name = if name.trim().is_empty() {
        format!("{} {}", method, clean_path)
    } else {
        name.trim().to_string()
    };
    let file_base = sanitize_filename(&api_name);
    let file_path = unique_path(dir, &file_base, ".json");
    let api_file = ApiFile {
        uuid: uuid::Uuid::new_v4().to_string(),
        name: api_name.clone(),
        method: method.clone(),
        path: clean_path.clone(),
        url,
        description: jmeter_child_string(el, "HTTPSampler.comments"),
        headers: headers.to_vec(),
        query,
        params,
        body,
        mock: MockConfig::default(),
        prescript: String::new(),
        examples: vec![],
        responses: vec![],
        doc_params: vec![],
        deprecated: false,
        protocol: api_protocol,
        order: None,
    };
    write_pretty(&file_path, &api_file)?;
        stats.add(&api_file.protocol);
    Ok(1)
}

// ==================== Eolink 导入 ====================
