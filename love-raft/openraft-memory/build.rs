fn main() {
    tonic_build::configure()
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        .compile_protos(&["proto/raft.proto", "proto/student.proto"], &["proto"])
        .unwrap();
}
