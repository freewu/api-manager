//! 由 export.rs 拆分：WADL
use super::*;
use super::raml::extract_base_url;
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

/// WADL 资源树节点：某路径段下的接口列表与子资源
struct WadlNode<'a> {
    apis: Vec<&'a ApiFile>,
    children: BTreeMap<String, WadlNode<'a>>,
}

/// 生成 WADL 文档（XML 字符串）
pub fn to_wadl(apis: &[(Vec<(String, bool)>, ApiFile)]) -> String {
    let mut tree = WadlNode {
        apis: Vec::new(),
        children: BTreeMap::new(),
    };
    let mut base = String::new();
    for (_, api) in apis {
        if api.protocol == "websocket" {
            continue;
        }
        let p = if !api.path.trim().is_empty() {
            api.path.trim().to_string()
        } else {
            api.url.trim().to_string()
        };
        if p.is_empty() {
            continue;
        }
        if base.is_empty() {
            base = extract_base_url(&api.url);
        }
        // 去掉 scheme://host 前缀，只保留路径段
        let path_only = p
            .split("://")
            .nth(1)
            .and_then(|r| r.split_once('/').map(|(_, rest)| format!("/{rest}")))
            .unwrap_or_else(|| p.clone());
        let segs: Vec<String> = path_only
            .split('/')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();
        let mut node = &mut tree;
        let n = segs.len();
        for (i, seg) in segs.iter().enumerate() {
            node = node.children.entry(seg.clone()).or_insert_with(|| WadlNode {
                apis: Vec::new(),
                children: BTreeMap::new(),
            });
            if i == n - 1 {
                node.apis.push(api);
            }
        }
    }
    let mut body = String::new();
    for (seg, node) in &tree.children {
        body.push_str(&wadl_node_xml(seg, node, 1));
    }
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<application xmlns=\"http://wadl.dev.java.net/2009/02\">\n  <resources base=\"{}\">\n{}{}</resources>\n</application>\n",
        xml_escape(&base),
        body,
        ""
    )
}

/// 递归生成 <resource> 元素
fn wadl_node_xml(seg: &str, node: &WadlNode, indent: usize) -> String {
    let pad = "  ".repeat(indent);
    let mut s = format!("{pad}<resource path=\"{}\">\n", xml_escape(seg));
    for api in &node.apis {
        s.push_str(&wadl_method_xml(api, indent + 1));
    }
    for (child_seg, child) in &node.children {
        s.push_str(&wadl_node_xml(child_seg, child, indent + 1));
    }
    s.push_str(&format!("{pad}</resource>\n"));
    s
}

/// 单个接口 → <method> 元素
fn wadl_method_xml(api: &ApiFile, indent: usize) -> String {
    let pad = "  ".repeat(indent);
    let mut s = format!("{pad}<method name=\"{}\">\n", xml_escape(&api.method));
    s.push_str(&format!("{pad}  <request>\n"));
    for h in api
        .headers
        .iter()
        .filter(|h| h.enabled && !h.key.trim().is_empty())
    {
        s.push_str(&format!(
            "{pad}    <param name=\"{}\" style=\"header\" type=\"xsd:string\"/>\n",
            xml_escape(&h.key)
        ));
    }
    for q in api
        .query
        .iter()
        .filter(|q| q.enabled && !q.key.trim().is_empty())
    {
        let default = if q.value.is_empty() {
            String::new()
        } else {
            format!(" default=\"{}\"", xml_escape(&q.value))
        };
        s.push_str(&format!(
            "{pad}    <param name=\"{}\" style=\"query\" type=\"xsd:string\"{}/>\n",
            xml_escape(&q.key),
            default
        ));
    }
    for p in api
        .params
        .iter()
        .filter(|p| p.enabled && !p.key.trim().is_empty())
    {
        s.push_str(&format!(
            "{pad}    <param name=\"{}\" style=\"path\" type=\"xsd:string\"/>\n",
            xml_escape(&p.key)
        ));
    }
    if matches!(api.body.mode.as_str(), "json" | "raw") && !api.body.raw.trim().is_empty() {
        s.push_str(&format!("{pad}    <representation mediaType=\"application/json\"/>\n"));
    }
    s.push_str(&format!("{pad}  </request>\n"));
    s.push_str(&format!("{pad}  <response status=\"200\"/>\n"));
    s.push_str(&format!("{pad}</method>\n"));
    s
}
