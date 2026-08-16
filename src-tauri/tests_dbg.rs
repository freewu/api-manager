#[test]
fn dbg_render_demo() {
    use std::fs;
    let p = r"E:\work\github\api-manager-examples\用户管理\创建用户.json";
    if let Ok(content) = fs::read_to_string(p) {
        let api: crate::ApiFile = serde_json::from_str(&content).unwrap();
        let md = crate::markdown::render(&api, "用户管理");
        println!("=====MD=====\n{md}\n=====END=====");
    }
}
