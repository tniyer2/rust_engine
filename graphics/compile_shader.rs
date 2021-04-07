
use shaderc::ShaderKind;

/// Compiles GLSL Source Code into a SPIR-V Binary.
pub fn compile_shader(source_text: &str, shader_kind: ShaderKind) -> Vec<u32> {
    let mut compiler = shaderc::Compiler::new().unwrap();

    let input_file = "unnamed";
    let entry_point = "main";
    let options = None;

    compiler
        .compile_into_spirv(
        	source_text, shader_kind,
        	input_file, entry_point, options)
        .expect("Failed to compile shader")
        .as_binary()
        .to_vec()
}
