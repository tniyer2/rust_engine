
use shaderc::ShaderKind;

/// Compiles GLSL Source Code into a SPIR-V Binary.
pub fn compile_shader(glsl: &str, shader_kind: ShaderKind) -> Vec<u32> {
    let mut compiler = shaderc::Compiler::new().unwrap();

    compiler
        .compile_into_spirv(glsl, shader_kind, "unnamed", "main", None)
        .expect("Failed to compile shader")
        .as_binary()
        .to_vec()
}
