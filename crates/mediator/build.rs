fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("../../proto/queue.proto")?;
    tonic_build::compile_protos("../../proto/fn.proto")?;
    Ok(())
}
