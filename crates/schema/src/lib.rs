pub mod schema_proto {
    tonic::include_proto!("schema_proto");
}

pub fn validate(schema: schema_proto::Schema) -> bool {
    false
}
