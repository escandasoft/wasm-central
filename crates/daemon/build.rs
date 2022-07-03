fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("../../proto/fn.proto")?;
    tonic_build::compile_protos("../../proto/mgmt.proto")?;
    Ok(())
}
