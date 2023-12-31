struct VertexInput{
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>
};
struct VertexOutput{
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@group(0) @binding(0)
var<uniform> position: vec2<f32>;

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput{
    var out: VertexOutput;
    out.clip_position = vec4(model.position.x + position.x, model.position.y + position.y, model.position.z, 1.0);
    out.color = model.color;
    return out;
}
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32>{
    return vec4(in.color, 1.0);
}