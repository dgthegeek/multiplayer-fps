#import bevy_pbr::mesh_view_bindings
#import bevy_pbr::mesh_bindings

@group(1) @binding(0)
var<uniform> color: vec4<f32>;

@vertex
fn vertex(
    #import bevy_pbr::mesh_vertex_input
    #import bevy_pbr::mesh_view_uniforms
) -> #import bevy_pbr::mesh_vertex_output {
    var out: #import bevy_pbr::mesh_vertex_output;
    out.position = view.view_proj * model.position;
    return out;
}

@fragment
fn fragment(
    #import bevy_pbr::mesh_vertex_output
) -> @location(0) vec4<f32> {
    if (instance.Instance_Type == 0u) {
        return vec4<f32>(0.8, 0.7, 0.6, 1.0);
    } else {
        return vec4<f32>(0.3, 0.3, 0.3, 1.0);
    }
}