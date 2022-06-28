fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("../../proto/ipc.proto")?;
    tonic_build::compile_protos("../../proto/queue.proto")?;
    Ok(())
}
