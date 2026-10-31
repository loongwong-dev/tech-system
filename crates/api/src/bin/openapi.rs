//! 输出 `OpenAPI` JSON；生成物写入仓库根目录 `openapi.json`。

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", api::openapi::openapi_json()?);
    Ok(())
}
