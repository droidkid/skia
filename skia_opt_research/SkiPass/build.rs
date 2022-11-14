use std::io::Result;
fn main() -> Result<()> {
    prost_build::compile_protos(&["../protos/ski_pass.proto"], &["../protos/"])?;
    Ok(())
}
