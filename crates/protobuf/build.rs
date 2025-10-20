fn main() {
    use protobuf_codegen::Codegen;

    Codegen::new()
        .pure()
        .cargo_out_dir("generated_with_pure")
        .input("src/protos/memory.proto")
        .include("src/protos")
        .run_from_script();
}
