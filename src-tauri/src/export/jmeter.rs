//! 由 export.rs 拆分：JMeter
use super::*;
#[allow(unused_imports)]
use crate::{read_api, read_info_file, sanitize_filename, ApiFile, BodyData, KeyValue, MockConfig, ENV_FILE, INFO_FILE};
#[allow(unused_imports)]
use serde_json::{json, Map, Value};
#[allow(unused_imports)]
use std::collections::BTreeMap;
#[allow(unused_imports)]
use std::fs;
#[allow(unused_imports)]
use std::path::{Path, PathBuf};

/// 生成 JMeter 测试计划 XML（.jmx）
pub fn to_jmeter(apis: &[(Vec<(String, bool)>, ApiFile)]) -> String {
    let mut root = PNode {
        name: "root".into(),
        apis: Vec::new(),
        children: BTreeMap::new(),
    };
    for (segs, api) in apis {
        let mut node = &mut root;
        for (s, _dep) in segs {
            node = node.children.entry(s.clone()).or_insert_with(|| PNode {
                name: s.clone(),
                apis: Vec::new(),
                children: BTreeMap::new(),
            });
        }
        node.apis.push(api);
    }
    let mut groups_xml = String::new();
    // 根级接口 → 「API Manager」线程组
    if !root.apis.is_empty() {
        groups_xml.push_str(&jmeter_thread_group("API Manager", &root.apis));
    }
    for (_, c) in &root.children {
        groups_xml.push_str(&jmeter_pnode_thread_group(c, &c.name));
    }
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str("<jmeterTestPlan version=\"1.2\" properties=\"5.0\" jmeter=\"5.6.3\">\n");
    out.push_str("  <hashTree>\n");
    out.push_str("    <TestPlan guiclass=\"TestPlanGui\" testclass=\"TestPlan\" testname=\"API Manager 导出\" enabled=\"true\">\n");
    out.push_str("      <elementProp name=\"TestPlan.user_defined_variables\" elementType=\"Arguments\" guiclass=\"ArgumentsPanel\" testclass=\"Arguments\" testname=\"用户定义的变量\" enabled=\"true\">\n");
    out.push_str("        <collectionProp name=\"Arguments.arguments\">\n");
    out.push_str("          <elementProp name=\"host\" elementType=\"Argument\">\n");
    out.push_str("            <stringProp name=\"Argument.name\">host</stringProp>\n");
    out.push_str("            <stringProp name=\"Argument.value\">http://localhost</stringProp>\n");
    out.push_str("            <stringProp name=\"Argument.metadata\">=</stringProp>\n");
    out.push_str("          </elementProp>\n");
    out.push_str("        </collectionProp>\n");
    out.push_str("      </elementProp>\n");
    out.push_str("      <stringProp name=\"TestPlan.user_define_classpath\"></stringProp>\n");
    out.push_str("    </TestPlan>\n");
    out.push_str("    <hashTree>\n");
    out.push_str(&groups_xml);
    out.push_str("    </hashTree>\n");
    out.push_str("  </hashTree>\n");
    out.push_str("</jmeterTestPlan>\n");
    out
}

/// 分组（含嵌套）→ 线程组（完整路径作 testname），嵌套分组再生成子线程组
fn jmeter_pnode_thread_group(n: &PNode, full_path: &str) -> String {
    let mut out = String::new();
    if !n.apis.is_empty() {
        out.push_str(&jmeter_thread_group(full_path, &n.apis));
    }
    for (_, c) in &n.children {
        let child_path = format!("{full_path} / {}", c.name);
        out.push_str(&jmeter_pnode_thread_group(c, &child_path));
    }
    out
}

/// 一组接口 → ThreadGroup + hashTree（每个接口前带独立 HeaderManager）
fn jmeter_thread_group(name: &str, apis: &[&ApiFile]) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "      <ThreadGroup guiclass=\"ThreadGroupGui\" testclass=\"ThreadGroup\" testname=\"{}\" enabled=\"true\">\n",
        xml_escape(name)
    ));
    out.push_str("        <stringProp name=\"ThreadGroup.on_sample_error\">continue</stringProp>\n");
    out.push_str("        <elementProp name=\"ThreadGroup.main_controller\" elementType=\"LoopController\" guiclass=\"LoopControlPanel\" testclass=\"LoopController\" testname=\"循环控制器\" enabled=\"true\">\n");
    out.push_str("          <boolProp name=\"LoopController.continue_forever\">false</boolProp>\n");
    out.push_str("          <stringProp name=\"LoopController.loops\">1</stringProp>\n");
    out.push_str("        </elementProp>\n");
    out.push_str("        <stringProp name=\"ThreadGroup.num_threads\">1</stringProp>\n");
    out.push_str("        <stringProp name=\"ThreadGroup.ramp_time\">1</stringProp>\n");
    out.push_str("        <boolProp name=\"ThreadGroup.scheduler\">false</boolProp>\n");
    out.push_str("        <stringProp name=\"ThreadGroup.duration\"></stringProp>\n");
    out.push_str("        <stringProp name=\"ThreadGroup.delay\"></stringProp>\n");
    out.push_str("      </ThreadGroup>\n");
    out.push_str("      <hashTree>\n");
    for api in apis {
        out.push_str(&jmeter_sampler(api));
    }
    out.push_str("      </hashTree>\n");
    out
}

/// 接口 → HeaderManager（如有头）+ HTTPSamplerProxy
fn jmeter_sampler(api: &ApiFile) -> String {
    let mut out = String::new();
    let enabled_headers: Vec<&KeyValue> = api
        .headers
        .iter()
        .filter(|h| h.enabled && !h.key.trim().is_empty())
        .collect();
    if !enabled_headers.is_empty() {
        out.push_str("        <HeaderManager guiclass=\"HeaderPanel\" testclass=\"HeaderManager\" testname=\"HTTP信息头管理器\" enabled=\"true\">\n");
        out.push_str("          <collectionProp name=\"HeaderManager.headers\">\n");
        for h in &enabled_headers {
            out.push_str(&format!(
                "            <elementProp name=\"\" elementType=\"Header\">\n              <stringProp name=\"Header.name\">{}</stringProp>\n              <stringProp name=\"Header.value\">{}</stringProp>\n            </elementProp>\n",
                xml_escape(&h.key),
                xml_escape(&h.value)
            ));
        }
        out.push_str("          </collectionProp>\n");
        out.push_str("        </HeaderManager>\n");
        out.push_str("        <hashTree/>\n");
    }
    let method = if api.method.is_empty() {
        "GET".to_string()
    } else {
        api.method.to_uppercase()
    };
    let is_ws = api.protocol == "websocket";
    let (domain, path) = if is_ws {
        // WS：domain 为空，path 放完整地址
        ("".to_string(), api.path.clone())
    } else {
        ("${host}".to_string(), jmeter_path_with_query(api))
    };
    out.push_str(&format!(
        "        <HTTPSamplerProxy guiclass=\"HttpSamplerGui\" testclass=\"HTTPSamplerProxy\" testname=\"{}\" enabled=\"true\">\n",
        xml_escape(&api.name)
    ));
    // Arguments：body
    let args_xml = match api.body.mode.as_str() {
        "json" => {
            if api.body.raw.trim().is_empty() {
                String::new()
            } else {
                format!(
                    "          <elementProp name=\"HTTPsampler.Arguments\" elementType=\"Arguments\" guiclass=\"HTTPArgumentsPanel\" testclass=\"Arguments\" testname=\"用户定义的变量\" enabled=\"true\">\n            <collectionProp name=\"Arguments.arguments\">\n              <elementProp name=\"\" elementType=\"HTTPArgument\">\n                <boolProp name=\"HTTPArgument.always_encode\">false</boolProp>\n                <stringProp name=\"Argument.value\">{}</stringProp>\n                <stringProp name=\"Argument.metadata\">=</stringProp>\n                <boolProp name=\"HTTPArgument.use_equals\">true</boolProp>\n              </elementProp>\n            </collectionProp>\n          </elementProp>\n",
                    xml_escape(&api.body.raw)
                )
            }
        }
        "form" => {
            let fields: Vec<String> = api
                .body
                .form
                .iter()
                .filter(|f| f.enabled && !f.key.trim().is_empty())
                .map(|f| {
                    format!(
                        "              <elementProp name=\"{}\" elementType=\"HTTPArgument\">\n                <boolProp name=\"HTTPArgument.always_encode\">false</boolProp>\n                <stringProp name=\"Argument.value\">{}</stringProp>\n                <stringProp name=\"Argument.metadata\">=</stringProp>\n                <boolProp name=\"HTTPArgument.use_equals\">false</boolProp>\n              </elementProp>\n",
                        xml_escape(&f.key),
                        xml_escape(&f.value)
                    )
                })
                .collect();
            if fields.is_empty() {
                String::new()
            } else {
                format!(
                    "          <elementProp name=\"HTTPsampler.Arguments\" elementType=\"Arguments\" guiclass=\"HTTPArgumentsPanel\" testclass=\"Arguments\" testname=\"用户定义的变量\" enabled=\"true\">\n            <collectionProp name=\"Arguments.arguments\">\n{}            </collectionProp>\n          </elementProp>\n",
                    fields.join("")
                )
            }
        }
        "raw" => {
            if api.body.raw.trim().is_empty() {
                String::new()
            } else {
                format!(
                    "          <boolProp name=\"HTTPSampler.postBodyRaw\">true</boolProp>\n          <stringProp name=\"HTTPSampler.raw_body\">{}</stringProp>\n",
                    xml_escape(&api.body.raw)
                )
            }
        }
        _ => String::new(),
    };
    out.push_str(&args_xml);
    out.push_str(&format!(
        "          <stringProp name=\"HTTPSampler.domain\">{}</stringProp>\n",
        xml_escape(&domain)
    ));
    out.push_str("          <stringProp name=\"HTTPSampler.port\"></stringProp>\n");
    out.push_str("          <stringProp name=\"HTTPSampler.protocol\"></stringProp>\n");
    out.push_str(&format!(
        "          <stringProp name=\"HTTPSampler.path\">{}</stringProp>\n",
        xml_escape(&path)
    ));
    out.push_str(&format!(
        "          <stringProp name=\"HTTPSampler.method\">{}</stringProp>\n",
        xml_escape(&method)
    ));
    out.push_str("          <boolProp name=\"HTTPSampler.follow_redirects\">true</boolProp>\n");
    out.push_str("          <boolProp name=\"HTTPSampler.use_keepalive\">true</boolProp>\n");
    out.push_str("        </HTTPSamplerProxy>\n");
    out.push_str("        <hashTree/>\n");
    out
}

/// 路径 + 查询参数拼成 JMeter path
fn jmeter_path_with_query(api: &ApiFile) -> String {
    let mut path = api.path.clone();
    let enabled_query: Vec<&KeyValue> = api
        .query
        .iter()
        .filter(|q| q.enabled && !q.key.trim().is_empty())
        .collect();
    if !enabled_query.is_empty() {
        if !path.contains('?') {
            path.push('?');
        } else if !path.ends_with('?') && !path.ends_with('&') {
            path.push('&');
        }
        let pairs: Vec<String> = enabled_query
            .iter()
            .map(|q| format!("{}={}", q.key, q.value))
            .collect();
        path.push_str(&pairs.join("&"));
    }
    path
}

// ==================== apiDoc 导出 ====================
