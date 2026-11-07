<style>
svg.dg { width:100%; height:auto; max-width:760px; display:block; margin:16px auto 6px; background:#17171a; border:1px solid #3a3a3a; border-radius:10px; }
.dg .node { fill:#242424; stroke:#4a4a4e; }
.dg .acc  { fill:#221d33; stroke:#a78bfa; stroke-width:1.5; }
.dg .good { fill:#1d2a24; stroke:#34d399; stroke-width:2; }
.dg .bad  { fill:#2a1c26; stroke:#fb7185; stroke-width:2; }
.dg .ext  { fill:#1b1726; stroke:#a78bfa; stroke-width:1.5; stroke-dasharray:6 4; }
.dg .t  { font-family:ui-monospace,Menlo,monospace; font-size:13px; fill:#e8e8ea; font-weight:600; }
.dg .ts { font-family:ui-monospace,Menlo,monospace; font-size:11px; fill:#e8e8ea; }
.dg .s  { font-family:-apple-system,'Helvetica Neue',Arial,sans-serif; font-size:10.5px; fill:#a3a3a8; }
.dg .sg { font-family:-apple-system,'Helvetica Neue',Arial,sans-serif; font-size:10.5px; fill:#8ee7bf; }
.dg .sr { font-family:-apple-system,'Helvetica Neue',Arial,sans-serif; font-size:10.5px; fill:#fda4af; }
.dg .sp { font-family:-apple-system,'Helvetica Neue',Arial,sans-serif; font-size:10.5px; fill:#c4b5fd; }
.dg .h  { font-family:-apple-system,'Helvetica Neue',Arial,sans-serif; font-size:10px; fill:#c4b5fd; letter-spacing:1.5px; font-weight:700; }
.dg .num { font-family:-apple-system,'Helvetica Neue',Arial,sans-serif; font-size:11px; fill:#131313; font-weight:700; }
.dg .m { text-anchor:middle; }
.dg .ar { stroke:#a78bfa; stroke-width:1.8; fill:none; }
.dg .ag { stroke:#34d399; stroke-width:1.8; fill:none; }
.dg .aw { stroke:#a3a3a8; stroke-width:1.5; fill:none; stroke-dasharray:5 4; }
.dg .ax { stroke:#fb7185; stroke-width:1.8; fill:none; stroke-dasharray:5 4; }
.dg .grid { stroke:#3a3a3a; stroke-width:1; }
p.cap { text-align:center; color:#a3a3a8; font-size:9.5pt; font-style:italic; margin:0 0 18px; }
</style>

# The renderer, one frame at a time

This is a walkthrough of Lantern's renderer for someone who wants to build one like it. It follows a single frame
from the game to the screen, names the file each step lives in, and ends with what this renderer leaves out and
where each missing feature would go.

Read `engine-structure.md` first for where the folders sit, and `application-and-game-loop.md` for when `Render` is
called.

## The shape in one picture

<svg class="dg" viewBox="0 0 760 330" xmlns="http://www.w3.org/2000/svg">
<defs>
<marker id="d1p" markerWidth="10" markerHeight="8" refX="8" refY="3" orient="auto"><path d="M0,0 L8,3 L0,6 Z" fill="#a78bfa"/></marker>
<marker id="d1g" markerWidth="10" markerHeight="8" refX="8" refY="3" orient="auto"><path d="M0,0 L8,3 L0,6 Z" fill="#a3a3a8"/></marker>
</defs>
<text x="100" y="28" class="h m">GAME CODE (NYX)</text>
<text x="300" y="28" class="h m">PLAIN DATA</text>
<text x="515" y="28" class="h m">INTERFACE</text>
<text x="675" y="28" class="h m">BACKENDS · GPU</text>
<rect x="20" y="50" width="160" height="56" rx="8" class="node"/>
<text x="100" y="74" class="t m">OnRender hook</text>
<text x="100" y="92" class="s m">camera, light, fog, extras</text>
<rect x="20" y="130" width="160" height="56" rx="8" class="node"/>
<text x="100" y="154" class="t m">entity world</text>
<text x="100" y="172" class="s m">ExtractDraws → DrawItems</text>
<rect x="20" y="210" width="160" height="56" rx="8" class="node"/>
<text x="100" y="234" class="t m">HUD, menus</text>
<text x="100" y="252" class="s m">overlay quads + text</text>
<line x1="180" y1="78" x2="218" y2="78" class="ar" marker-end="url(#d1p)"/>
<line x1="180" y1="158" x2="218" y2="158" class="ar" marker-end="url(#d1p)"/>
<line x1="180" y1="238" x2="218" y2="238" class="ar" marker-end="url(#d1p)"/>
<rect x="220" y="44" width="160" height="230" rx="10" class="acc"/>
<text x="300" y="68" class="t m">RenderWorld</text>
<text x="300" y="84" class="s m">rebuilt every frame</text>
<rect x="234" y="96" width="132" height="22" rx="4" class="node"/><text x="300" y="111" class="ts m">DrawItem[ ]</text>
<rect x="234" y="124" width="132" height="22" rx="4" class="node"/><text x="300" y="139" class="ts m">LineVertex[ ]</text>
<rect x="234" y="152" width="132" height="22" rx="4" class="node"/><text x="300" y="167" class="ts m">Overlay vertices</text>
<rect x="234" y="180" width="132" height="22" rx="4" class="node"/><text x="300" y="195" class="ts m">Camera</text>
<rect x="234" y="208" width="132" height="22" rx="4" class="node"/><text x="300" y="223" class="ts m">DirectionalLight</text>
<rect x="234" y="236" width="132" height="22" rx="4" class="node"/><text x="300" y="251" class="ts m">clear colour · fog</text>
<line x1="380" y1="158" x2="448" y2="158" class="ar" marker-end="url(#d1p)"/>
<text x="414" y="150" class="sp m">Render()</text>
<rect x="450" y="126" width="130" height="64" rx="8" class="good"/>
<text x="515" y="152" class="t m">RenderBackend</text>
<text x="515" y="170" class="sg m">7 methods</text>
<path d="M580,145 C595,145 595,68 608,68" class="ar" marker-end="url(#d1p)"/>
<path d="M580,171 C595,171 595,268 608,268" class="ar" marker-end="url(#d1p)"/>
<rect x="610" y="40" width="130" height="56" rx="8" class="node"/>
<text x="675" y="64" class="t m">GpuRenderer</text>
<text x="675" y="82" class="s m">sdl-facade/renderer</text>
<line x1="675" y1="96" x2="675" y2="116" class="aw" marker-end="url(#d1g)"/>
<rect x="610" y="118" width="130" height="36" rx="8" class="node"/>
<text x="675" y="141" class="t m">SDL GPU</text>
<line x1="675" y1="154" x2="675" y2="174" class="aw" marker-end="url(#d1g)"/>
<rect x="610" y="176" width="130" height="36" rx="8" class="node"/>
<text x="675" y="199" class="s m">Vulkan · Metal · D3D12</text>
<rect x="610" y="240" width="130" height="56" rx="8" class="node"/>
<text x="675" y="264" class="t m">HeadlessRenderer</text>
<text x="675" y="282" class="s m">no GPU — tests</text>
<text x="380" y="318" class="s m">Nothing left of the green box ever calls a GPU function. Everything right of it is swappable.</text>
</svg>
<p class="cap">Figure 1 — The frame flows left to right as data. Only the backends know a GPU exists.</p>

Two ideas carry the whole design:

1. **The frame is data.** Nothing above the renderer calls a GPU function. Each frame, the game fills a
   `RenderWorld`: a list of draw items, a list of debug lines, a list of 2D overlay vertices, and the camera, light,
   clear colour and fog. The renderer reads it and forgets it.
2. **One small interface between "what" and "how".** `RenderBackend` (`engine/src/renderer/RenderBackend.nox`) has
   seven methods: its driver name, create and remove a mesh, create and remove a texture, render a frame, and
   capture a frame to memory. `GpuRenderer` implements it over SDL GPU; `HeadlessRenderer` implements it with no
   GPU at all, for tests. A Vulkan or Direct3D renderer written by hand would be a third implementation, and nothing
   above the interface would change.

<svg class="dg" viewBox="0 0 760 150" xmlns="http://www.w3.org/2000/svg">
<rect x="190" y="20" width="380" height="114" rx="10" class="good"/>
<text x="380" y="42" class="t m">interface RenderBackend</text>
<text x="208" y="64" class="ts">DriverName()</text>
<text x="208" y="82" class="ts">CreateMesh(mesh)</text>
<text x="208" y="100" class="ts">RemoveMesh(mesh)</text>
<text x="208" y="118" class="ts">CreateTexture(texture)</text>
<text x="420" y="64" class="ts">RemoveTexture(texture)</text>
<text x="420" y="82" class="ts">Render(world)</text>
<text x="420" y="100" class="ts">Capture(world, width,</text>
<text x="420" y="116" class="ts">        height)</text>
<text x="95" y="60" class="sp m">"what" lives above</text>
<text x="95" y="78" class="s m">game, entities, UI,</text>
<text x="95" y="94" class="s m">RenderWorld</text>
<text x="645" y="60" class="sp m">"how" lives below</text>
<text x="645" y="78" class="s m">devices, buffers,</text>
<text x="645" y="94" class="s m">pipelines, shaders</text>
</svg>
<p class="cap">Figure 2 — The seven methods are the whole contract. Every method except <code>DriverName</code> returns a <code>Result</code> or nothing; see <code>RenderBackend.nox</code> for the full signatures.</p>

## The files

The renderer is split in two folders. `renderer/` holds platform-neutral data and maths; `sdl-facade/renderer/`
holds everything that touches SDL GPU.

<svg class="dg" viewBox="0 0 760 300" xmlns="http://www.w3.org/2000/svg">
<rect x="16" y="16" width="356" height="268" rx="10" class="acc"/>
<text x="194" y="38" class="t m">renderer/</text>
<text x="194" y="54" class="s m">platform-neutral: data, maths, assets</text>
<rect x="30" y="66" width="160" height="40" rx="6" class="good"/><text x="110" y="84" class="ts m">RenderBackend</text><text x="110" y="99" class="s m">interface, RenderReport</text>
<rect x="198" y="66" width="160" height="40" rx="6" class="node"/><text x="278" y="84" class="ts m">RenderWorld</text><text x="278" y="99" class="s m">one frame of data</text>
<rect x="30" y="114" width="160" height="40" rx="6" class="node"/><text x="110" y="132" class="ts m">Camera · Frustum</text><text x="110" y="147" class="s m">matrices, culling planes</text>
<rect x="198" y="114" width="160" height="40" rx="6" class="node"/><text x="278" y="132" class="ts m">Mesh · Primitives</text><text x="278" y="147" class="s m">Vertex, built-in shapes</text>
<rect x="30" y="162" width="160" height="40" rx="6" class="node"/><text x="110" y="180" class="ts m">Texture · Png</text><text x="110" y="195" class="s m">RGBA8 pixels, PNG I/O</text>
<rect x="198" y="162" width="160" height="40" rx="6" class="node"/><text x="278" y="180" class="ts m">GltfLoader</text><text x="278" y="195" class="s m">.glb/.gltf → MeshData</text>
<rect x="30" y="210" width="160" height="40" rx="6" class="node"/><text x="110" y="228" class="ts m">FontData · Text</text><text x="110" y="243" class="s m">8x8 font, overlay quads</text>
<rect x="198" y="210" width="160" height="40" rx="6" class="node"/><text x="278" y="228" class="ts m">HeadlessRenderer</text><text x="278" y="243" class="s m">backend with no GPU</text>
<rect x="388" y="16" width="356" height="268" rx="10" class="node"/>
<text x="566" y="38" class="t m">sdl-facade/renderer/</text>
<text x="566" y="54" class="s m">everything that talks to SDL GPU</text>
<rect x="402" y="66" width="328" height="40" rx="6" class="good"/><text x="566" y="84" class="ts m">GpuRenderer</text><text x="566" y="99" class="s m">implements RenderBackend; the frame itself</text>
<rect x="402" y="114" width="160" height="40" rx="6" class="node"/><text x="482" y="132" class="ts m">GpuDevice</text><text x="482" y="147" class="s m">options, depth, swapchain</text>
<rect x="570" y="114" width="160" height="40" rx="6" class="node"/><text x="650" y="132" class="ts m">GpuPipelines</text><text x="650" y="147" class="s m">lit, lines, overlay</text>
<rect x="402" y="162" width="160" height="40" rx="6" class="node"/><text x="482" y="180" class="ts m">GpuShaders</text><text x="482" y="195" class="s m">load SPIR-V / MSL</text>
<rect x="570" y="162" width="160" height="40" rx="6" class="node"/><text x="650" y="180" class="ts m">GpuResources</text><text x="650" y="195" class="s m">meshes, textures, uploads</text>
<rect x="402" y="210" width="160" height="40" rx="6" class="node"/><text x="482" y="228" class="ts m">GpuUniforms</text><text x="482" y="243" class="s m">std140 uniform blocks</text>
<rect x="570" y="210" width="160" height="40" rx="6" class="node"/><text x="650" y="228" class="ts m">GpuDrawList</text><text x="650" y="243" class="s m">draw order</text>
<text x="566" y="272" class="s m">+ engine/shaders/*.vert, *.frag → compiled/</text>
</svg>
<p class="cap">Figure 3 — The two halves of the renderer.</p>

| File | What it holds |
| --- | --- |
| `renderer/RenderBackend.nox` | The `RenderBackend` interface and `RenderReport` |
| `renderer/RenderWorld.nox` | `RenderWorld` (one frame), `DrawItem`, `DirectionalLight`, `LineVertex`, the 2D `Overlay` |
| `renderer/Camera.nox` | `Camera`: position, forward, field of view, near/far planes, view and projection matrices |
| `renderer/Frustum.nox` | The camera's view volume as six planes, and the box test used for culling |
| `renderer/Mesh.nox` | `Vertex` (48 bytes: position, normal, uv, colour) and `MeshData` (vertices and indices) |
| `renderer/Texture.nox`, `Png.nox` | `TextureData` (RGBA8 pixels, filter, wrap), PNG reading and writing |
| `renderer/Primitives.nox` | Cube, box, plane, ramp, capsule, sphere and gem meshes built in code |
| `renderer/GltfLoader.nox` | A `.glb`/`.gltf` file flattened into one `MeshData` |
| `renderer/FontData.nox`, `Text.nox` | An 8x8 bitmap font baked into a 128x48 atlas, and text laid out as overlay quads |
| `renderer/HeadlessRenderer.nox` | A `RenderBackend` with no GPU |
| `sdl-facade/renderer/GpuRenderer.nox` | `GpuRenderer`: the `RenderBackend` over SDL GPU; the frame itself |
| `sdl-facade/renderer/GpuDevice.nox` | Device options, the depth format and the swapchain set-up |
| `sdl-facade/renderer/GpuPipelines.nox` | The three pipelines: lit meshes, lines, overlay |
| `sdl-facade/renderer/GpuShaders.nox` | Finding and loading the compiled shaders (SPIR-V or MSL) |
| `sdl-facade/renderer/GpuResources.nox` | GPU meshes and textures, samplers, uploads, mipmaps, growable vertex buffers |
| `sdl-facade/renderer/GpuUniforms.nox` | The uniform blocks, laid out exactly as the shaders declare them |
| `sdl-facade/renderer/GpuDrawList.nox` | The order meshes are drawn in |
| `engine/shaders/*.vert`, `*.frag` | The GLSL shaders; `engine/shaders/compiled/` holds them built for Vulkan and Metal |

## 1. Start-up: device, pipelines, default resources

`GpuRenderer:::Create(window, options)` (`GpuRenderer.nox`) does the one-time work, in five steps:

<svg class="dg" viewBox="0 0 760 190" xmlns="http://www.w3.org/2000/svg">
<defs>
<marker id="d4p" markerWidth="10" markerHeight="8" refX="8" refY="3" orient="auto"><path d="M0,0 L8,3 L0,6 Z" fill="#a78bfa"/></marker>
</defs>
<g>
<rect x="12" y="34" width="128" height="104" rx="8" class="node"/>
<circle cx="32" cy="34" r="11" fill="#a78bfa"/><text x="32" y="38" class="num m">1</text>
<text x="76" y="66" class="t m">Open device</text>
<text x="76" y="86" class="s m">SDL picks Vulkan,</text>
<text x="76" y="100" class="s m">Metal or D3D12</text>
<text x="76" y="122" class="sp m">SPIR-V or MSL</text>
</g>
<line x1="140" y1="86" x2="158" y2="86" class="ar" marker-end="url(#d4p)"/>
<g>
<rect x="160" y="34" width="128" height="104" rx="8" class="node"/>
<circle cx="180" cy="34" r="11" fill="#a78bfa"/><text x="180" y="38" class="num m">2</text>
<text x="224" y="66" class="t m">Claim window</text>
<text x="224" y="86" class="s m">device gets a</text>
<text x="224" y="100" class="s m">swapchain</text>
<text x="224" y="122" class="sp m">images to show</text>
</g>
<line x1="288" y1="86" x2="306" y2="86" class="ar" marker-end="url(#d4p)"/>
<g>
<rect x="308" y="34" width="128" height="104" rx="8" class="node"/>
<circle cx="328" cy="34" r="11" fill="#a78bfa"/><text x="328" y="38" class="num m">3</text>
<text x="372" y="66" class="t m">Pick formats</text>
<text x="372" y="86" class="s m">colour: SDR_LINEAR</text>
<text x="372" y="100" class="s m">depth: best precision</text>
<text x="372" y="122" class="sp m">D32 float first</text>
</g>
<line x1="436" y1="86" x2="454" y2="86" class="ar" marker-end="url(#d4p)"/>
<g>
<rect x="456" y="34" width="128" height="104" rx="8" class="node"/>
<circle cx="476" cy="34" r="11" fill="#a78bfa"/><text x="476" y="38" class="num m">4</text>
<text x="520" y="66" class="t m">3 pipelines</text>
<text x="520" y="86" class="s m">lit · lines · overlay</text>
<text x="520" y="100" class="s m">shaders + state</text>
<text x="520" y="122" class="sp m">GpuPipelines</text>
</g>
<line x1="584" y1="86" x2="602" y2="86" class="ar" marker-end="url(#d4p)"/>
<g>
<rect x="604" y="34" width="144" height="104" rx="8" class="node"/>
<circle cx="624" cy="34" r="11" fill="#a78bfa"/><text x="624" y="38" class="num m">5</text>
<text x="676" y="66" class="t m">Defaults</text>
<text x="676" y="86" class="s m">4 samplers</text>
<text x="676" y="100" class="s m">1x1 white texture</text>
<text x="676" y="122" class="sp m">GpuResources</text>
</g>
<text x="380" y="170" class="s m">CreateOffscreen runs the same steps but skips step 2 — there is no window, so no swapchain.</text>
</svg>
<p class="cap">Figure 4 — Start-up, once per renderer.</p>

1. **Open a device** (`OpenDevice`). SDL GPU chooses Vulkan, Metal or Direct3D 12 for the platform. The renderer asks
   for a device that accepts SPIR-V or MSL shaders, since those are the two compiled forms it ships.
2. **Claim the window** for the device, which gives it a swapchain: the images the window shows.
3. **Pick formats** (`GpuDevice.nox`). Colour comes from the swapchain. The swapchain is set to SDR_LINEAR where the
   window supports it, so the hardware converts linear colour to sRGB as pixels are written; otherwise the shaders
   do the conversion themselves. Depth takes the most precise format the card has (32-bit float first).
4. **Build three pipelines** (`GpuPipelines.nox`). A pipeline fixes everything about how one kind of geometry is
   drawn: its shaders, its vertex layout, the primitive type, culling, depth testing and blending.
5. **Make the samplers and the white texture** (`GpuResources.nox`). There are four samplers (linear or nearest,
   repeat or clamp) and every texture uses the one matching its settings. A 1x1 white texture stands in wherever a
   draw has no texture, so the shader can always sample one and multiply.

`GpuRenderer:::CreateOffscreen` does the same without a window; the editor's viewport and the GPU tests use it.

### Why SDR_LINEAR matters

Lighting maths only works in *linear* colour, where doubling a number doubles the light. Screens expect *sRGB*,
which spends more of its bits on dark tones. Someone has to convert between the two, exactly once:

<svg class="dg" viewBox="0 0 760 200" xmlns="http://www.w3.org/2000/svg">
<defs>
<linearGradient id="d5lin" x1="0" x2="1" y1="0" y2="0"><stop offset="0" stop-color="#000"/><stop offset="0.25" stop-color="#898989"/><stop offset="0.5" stop-color="#bcbcbc"/><stop offset="0.75" stop-color="#e1e1e1"/><stop offset="1" stop-color="#fff"/></linearGradient>
<linearGradient id="d5srgb" x1="0" x2="1" y1="0" y2="0"><stop offset="0" stop-color="#000"/><stop offset="1" stop-color="#fff"/></linearGradient>
<marker id="d5p" markerWidth="10" markerHeight="8" refX="8" refY="3" orient="auto"><path d="M0,0 L8,3 L0,6 Z" fill="#a78bfa"/></marker>
<marker id="d5g" markerWidth="10" markerHeight="8" refX="8" refY="3" orient="auto"><path d="M0,0 L8,3 L0,6 Z" fill="#a3a3a8"/></marker>
</defs>
<rect x="20" y="30" width="150" height="56" rx="8" class="node"/>
<text x="95" y="54" class="t m">lit.frag</text>
<text x="95" y="72" class="s m">outputs linear colour</text>
<rect x="300" y="18" width="190" height="40" rx="8" class="good"/>
<text x="395" y="36" class="ts m">SDR_LINEAR swapchain</text>
<text x="395" y="50" class="sg m">hardware encodes to sRGB</text>
<rect x="300" y="80" width="190" height="40" rx="8" class="node"/>
<text x="395" y="98" class="ts m">plain swapchain</text>
<text x="395" y="112" class="s m">shader encodes (sRGB flag)</text>
<path d="M170,50 C230,50 240,38 298,38" class="ar" marker-end="url(#d5p)"/>
<path d="M170,66 C230,66 240,100 298,100" class="aw" marker-end="url(#d5g)"/>
<rect x="600" y="40" width="140" height="56" rx="8" class="node"/>
<text x="670" y="64" class="t m">screen</text>
<text x="670" y="82" class="s m">expects sRGB</text>
<line x1="490" y1="38" x2="598" y2="60" class="ar" marker-end="url(#d5p)"/>
<line x1="490" y1="100" x2="598" y2="78" class="aw" marker-end="url(#d5g)"/>
<text x="20" y="150" class="s">linear 0 → 1</text>
<rect x="120" y="138" width="300" height="18" fill="url(#d5srgb)"/>
<text x="20" y="180" class="s">after sRGB encode</text>
<rect x="120" y="168" width="300" height="18" fill="url(#d5lin)"/>
<text x="440" y="152" class="s">The same ramp before and after encoding.</text>
<text x="440" y="168" class="s">Encoding lifts the darks so the screen's response</text>
<text x="440" y="184" class="s">curve brings them back to the intended brightness.</text>
</svg>
<p class="cap">Figure 5 — Either the swapchain or the shader does the conversion. Never both, never neither.</p>

### The three pipelines

| Pipeline | Draws | Vertex | Primitive | Back-face culling | Depth test / write | Blending |
| --- | --- | --- | --- | --- | --- | --- |
| `lit` | meshes | 48 bytes: position, normal, uv, colour | triangles | yes | yes / yes | off |
| `lines` | debug lines | 28 bytes: position, colour | lines | no | yes / no | alpha |
| `overlay` | HUD, menus, text | 32 bytes: 2D position, uv, colour | triangles | no | no / no | alpha |

Each pipeline's shaders are loaded from `engine/shaders/compiled/` (`GpuShaders.nox`): `.spv` for Vulkan, `.msl`
for Metal. Here is what each vertex looks like in memory, byte by byte:

<svg class="dg" viewBox="0 0 760 230" xmlns="http://www.w3.org/2000/svg">
<text x="20" y="62" class="t">lit</text><text x="20" y="78" class="s">48 bytes</text>
<rect x="140" y="46" width="144" height="36" fill="#2a2340" stroke="#a78bfa"/><text x="212" y="62" class="ts m">position</text><text x="212" y="76" class="s m">vec3 · 12</text>
<rect x="284" y="46" width="144" height="36" fill="#1d2a24" stroke="#34d399"/><text x="356" y="62" class="ts m">normal</text><text x="356" y="76" class="s m">vec3 · 12</text>
<rect x="428" y="46" width="96" height="36" fill="#2e2717" stroke="#fbbf24"/><text x="476" y="62" class="ts m">uv</text><text x="476" y="76" class="s m">vec2 · 8</text>
<rect x="524" y="46" width="192" height="36" fill="#2e1d29" stroke="#f0a3d0"/><text x="620" y="62" class="ts m">colour</text><text x="620" y="76" class="s m">vec4 · 16</text>
<text x="20" y="122" class="t">lines</text><text x="20" y="138" class="s">28 bytes</text>
<rect x="140" y="106" width="144" height="36" fill="#2a2340" stroke="#a78bfa"/><text x="212" y="122" class="ts m">position</text><text x="212" y="136" class="s m">vec3 · 12</text>
<rect x="284" y="106" width="192" height="36" fill="#2e1d29" stroke="#f0a3d0"/><text x="380" y="122" class="ts m">colour</text><text x="380" y="136" class="s m">vec4 · 16</text>
<text x="20" y="182" class="t">overlay</text><text x="20" y="198" class="s">32 bytes</text>
<rect x="140" y="166" width="96" height="36" fill="#2a2340" stroke="#a78bfa"/><text x="188" y="182" class="ts m">position</text><text x="188" y="196" class="s m">vec2 · 8</text>
<rect x="236" y="166" width="96" height="36" fill="#2e2717" stroke="#fbbf24"/><text x="284" y="182" class="ts m">uv</text><text x="284" y="196" class="s m">vec2 · 8</text>
<rect x="332" y="166" width="192" height="36" fill="#2e1d29" stroke="#f0a3d0"/><text x="428" y="182" class="ts m">colour</text><text x="428" y="196" class="s m">vec4 · 16</text>
<line x1="140" y1="24" x2="716" y2="24" class="grid"/>
<line x1="140" y1="20" x2="140" y2="28" class="grid"/><text x="140" y="16" class="s m">0</text>
<line x1="284" y1="20" x2="284" y2="28" class="grid"/><text x="284" y="16" class="s m">12</text>
<line x1="428" y1="20" x2="428" y2="28" class="grid"/><text x="428" y="16" class="s m">24</text>
<line x1="524" y1="20" x2="524" y2="28" class="grid"/><text x="524" y="16" class="s m">32</text>
<line x1="716" y1="20" x2="716" y2="28" class="grid"/><text x="716" y="16" class="s m">48</text>
</svg>
<p class="cap">Figure 6 — Vertex layouts at 12 px per byte. Every field is <code>f32</code>: <code>Vertex</code>, <code>LineVertex</code> and <code>OverlayVertex</code>.</p>

The pipeline settings follow from what each layer is for. Meshes are solid, so they cull back faces and write depth.
Lines are thin and debug-only, so they test against mesh depth (a line behind a wall is hidden) but write none.
The overlay is flat UI on top of everything, so it ignores depth altogether.

## 2. Long-lived resources: meshes and textures

A game uploads a mesh or texture once and refers to it by handle afterwards.

- `CreateMesh(MeshData)` validates the indices, creates a vertex buffer and an index buffer on the GPU, copies the
  data through a *transfer buffer* (memory the CPU can write and the GPU can read) in a copy pass, and keeps the
  mesh's bounding box for culling (`UploadMesh`).
- `CreateTexture(TextureData)` uploads the pixels the same way into an sRGB texture. A texture with linear filtering
  also gets a full mipmap chain generated on the GPU, so it doesn't shimmer when seen from far away
  (`UploadTexture`, `MipLevelCount`).

<svg class="dg" viewBox="0 0 760 250" xmlns="http://www.w3.org/2000/svg">
<defs>
<marker id="d7p" markerWidth="10" markerHeight="8" refX="8" refY="3" orient="auto"><path d="M0,0 L8,3 L0,6 Z" fill="#a78bfa"/></marker>
</defs>
<text x="105" y="22" class="h m">CPU MEMORY</text>
<text x="370" y="22" class="h m">SHARED</text>
<text x="640" y="22" class="h m">GPU MEMORY</text>
<line x1="230" y1="30" x2="230" y2="236" class="grid" stroke-dasharray="3 4"/>
<line x1="510" y1="30" x2="510" y2="236" class="grid" stroke-dasharray="3 4"/>
<rect x="20" y="44" width="170" height="60" rx="8" class="node"/>
<text x="105" y="68" class="t m">MeshData</text>
<text x="105" y="86" class="s m">vertices + indices</text>
<rect x="270" y="44" width="200" height="60" rx="8" class="acc"/>
<text x="370" y="68" class="t m">transfer buffer</text>
<text x="370" y="86" class="s m">CPU writes, GPU reads</text>
<rect x="550" y="38" width="190" height="34" rx="6" class="good"/><text x="645" y="60" class="ts m">vertex buffer</text>
<rect x="550" y="78" width="190" height="34" rx="6" class="good"/><text x="645" y="100" class="ts m">index buffer</text>
<line x1="190" y1="74" x2="268" y2="74" class="ar" marker-end="url(#d7p)"/><text x="230" y="66" class="sp m">copy</text>
<path d="M470,66 C510,66 510,55 548,55" class="ar" marker-end="url(#d7p)"/>
<path d="M470,82 C510,82 510,95 548,95" class="ar" marker-end="url(#d7p)"/>
<text x="510" y="126" class="sp m">copy pass</text>
<text x="105" y="122" class="s m">+ bounding box kept for culling</text>
<rect x="20" y="150" width="170" height="60" rx="8" class="node"/>
<text x="105" y="174" class="t m">TextureData</text>
<text x="105" y="192" class="s m">RGBA8, filter, wrap</text>
<rect x="270" y="150" width="200" height="60" rx="8" class="acc"/>
<text x="370" y="174" class="t m">transfer buffer</text>
<text x="370" y="192" class="s m">pixels</text>
<line x1="190" y1="180" x2="268" y2="180" class="ar" marker-end="url(#d7p)"/>
<line x1="470" y1="180" x2="548" y2="180" class="ar" marker-end="url(#d7p)"/>
<rect x="550" y="146" width="64" height="64" fill="#1d2a24" stroke="#34d399" stroke-width="2"/>
<rect x="620" y="178" width="32" height="32" fill="#1d2a24" stroke="#34d399"/>
<rect x="658" y="194" width="16" height="16" fill="#1d2a24" stroke="#34d399"/>
<rect x="680" y="202" width="8" height="8" fill="#1d2a24" stroke="#34d399"/>
<rect x="694" y="206" width="4" height="4" fill="#34d399"/>
<text x="582" y="182" class="s m">sRGB</text>
<text x="660" y="230" class="sg m">mip chain, generated on the GPU</text>
<text x="660" y="243" class="s m">(linear-filtered textures only)</text>
</svg>
<p class="cap">Figure 7 — Every upload goes CPU → transfer buffer → copy pass → GPU-only memory.</p>

A mip chain is the same image at half size, quarter size and so on down to 1x1. When a texture covers only a few
pixels on screen, the GPU samples a small mip instead of skipping across the full-size image, which is what causes
shimmer.

- Both live in a `Pool`, which hands back a *generational handle*: an index and a generation number. Removing a mesh
  bumps its slot's generation, so an old handle stops matching. A draw that still holds it is skipped with a warning
  instead of reading freed memory (`core/Handle.nox`).

<svg class="dg" viewBox="0 0 760 240" xmlns="http://www.w3.org/2000/svg">
<defs>
<marker id="d8g" markerWidth="10" markerHeight="8" refX="8" refY="3" orient="auto"><path d="M0,0 L8,3 L0,6 Z" fill="#34d399"/></marker>
<marker id="d8r" markerWidth="10" markerHeight="8" refX="8" refY="3" orient="auto"><path d="M0,0 L8,3 L0,6 Z" fill="#fb7185"/></marker>
</defs>
<text x="185" y="24" class="h m">BEFORE RemoveMesh</text>
<text x="575" y="24" class="h m">AFTER RemoveMesh</text>
<line x1="380" y1="14" x2="380" y2="226" class="grid" stroke-dasharray="3 4"/>
<rect x="125" y="38" width="120" height="44" rx="8" class="good"/>
<text x="185" y="58" class="ts m">handle</text>
<text x="185" y="74" class="sg m">index 2 · gen 3</text>
<rect x="30" y="118" width="58" height="62" rx="6" class="node"/><text x="59" y="138" class="s m">slot 0</text><text x="59" y="160" class="ts m">gen 1</text>
<rect x="94" y="118" width="58" height="62" rx="6" class="node"/><text x="123" y="138" class="s m">slot 1</text><text x="123" y="160" class="ts m">gen 1</text>
<rect x="158" y="118" width="58" height="62" rx="6" class="good"/><text x="187" y="138" class="sg m">slot 2</text><text x="187" y="160" class="ts m">gen 3</text>
<rect x="222" y="118" width="58" height="62" rx="6" class="node"/><text x="251" y="138" class="s m">slot 3</text><text x="251" y="160" class="ts m">gen 2</text>
<rect x="286" y="118" width="58" height="62" rx="6" class="node"/><text x="315" y="138" class="s m">slot 4</text><text x="315" y="160" class="ts m">gen 1</text>
<line x1="186" y1="82" x2="187" y2="116" class="ag" marker-end="url(#d8g)"/>
<text x="198" y="104" class="sg">3 = 3 ✓</text>
<text x="187" y="206" class="sg m">generations match → mesh is drawn</text>
<rect x="515" y="38" width="120" height="44" rx="8" class="bad"/>
<text x="575" y="58" class="ts m">same old handle</text>
<text x="575" y="74" class="sr m">index 2 · gen 3</text>
<rect x="420" y="118" width="58" height="62" rx="6" class="node"/><text x="449" y="138" class="s m">slot 0</text><text x="449" y="160" class="ts m">gen 1</text>
<rect x="484" y="118" width="58" height="62" rx="6" class="node"/><text x="513" y="138" class="s m">slot 1</text><text x="513" y="160" class="ts m">gen 1</text>
<rect x="548" y="118" width="58" height="62" rx="6" class="bad"/><text x="577" y="138" class="sr m">slot 2</text><text x="577" y="160" class="ts m">gen 4</text><text x="577" y="174" class="s m">freed</text>
<rect x="612" y="118" width="58" height="62" rx="6" class="node"/><text x="641" y="138" class="s m">slot 3</text><text x="641" y="160" class="ts m">gen 2</text>
<rect x="676" y="118" width="58" height="62" rx="6" class="node"/><text x="705" y="138" class="s m">slot 4</text><text x="705" y="160" class="ts m">gen 1</text>
<line x1="576" y1="82" x2="577" y2="116" class="ax" marker-end="url(#d8r)"/>
<text x="588" y="104" class="sr">3 ≠ 4 ✗</text>
<text x="577" y="206" class="sr m">mismatch → draw skipped with a warning</text>
<text x="577" y="222" class="s m">(never reads freed memory)</text>
</svg>
<p class="cap">Figure 8 — A generational handle. Slot contents are illustrative. Generations start at 0, and a freed slot is reused later with its bumped generation, so a new handle never matches an old one.</p>

## 3. The game fills a RenderWorld

Each frame, before anything touches the GPU, the game describes the frame (`host/GameHost.nox`, `Render`):

1. `renderWorld->Clear()` empties last frame's lists.
2. The game's `OnRender` hook sets the camera, light and fog, and may add draws, lines and overlay text directly.
3. The game's UI draws its boxes and text as overlay quads.
4. `ExtractDraws` (`engine/entities/World.nox`) walks every entity with a visible mesh renderer and adds a
   `DrawItem` for it: mesh handle, texture handle, world transform and tint colour. The transform is interpolated
   between the last two simulation steps with the loop's `alpha`, which is what keeps motion smooth when the display
   rate and the 60 Hz simulation don't line up (see `application-and-game-loop.md`).
5. The debug HUD adds its panel to the overlay.
6. `renderer->Render(renderWorld)`.

<svg class="dg" viewBox="0 0 760 330" xmlns="http://www.w3.org/2000/svg">
<defs>
<marker id="d9p" markerWidth="10" markerHeight="8" refX="8" refY="3" orient="auto"><path d="M0,0 L8,3 L0,6 Z" fill="#a78bfa"/></marker>
</defs>
<rect x="20" y="30" width="300" height="36" rx="6" class="node"/><circle cx="40" cy="48" r="10" fill="#a78bfa"/><text x="40" y="52" class="num m">1</text><text x="60" y="53" class="ts">renderWorld->Clear()</text>
<rect x="20" y="76" width="300" height="36" rx="6" class="node"/><circle cx="40" cy="94" r="10" fill="#a78bfa"/><text x="40" y="98" class="num m">2</text><text x="60" y="99" class="ts">game OnRender hook</text>
<rect x="20" y="122" width="300" height="36" rx="6" class="node"/><circle cx="40" cy="140" r="10" fill="#a78bfa"/><text x="40" y="144" class="num m">3</text><text x="60" y="145" class="ts">game UI</text>
<rect x="20" y="168" width="300" height="36" rx="6" class="node"/><circle cx="40" cy="186" r="10" fill="#a78bfa"/><text x="40" y="190" class="num m">4</text><text x="60" y="191" class="ts">ExtractDraws (entity world)</text>
<rect x="20" y="214" width="300" height="36" rx="6" class="node"/><circle cx="40" cy="232" r="10" fill="#a78bfa"/><text x="40" y="236" class="num m">5</text><text x="60" y="237" class="ts">debug HUD</text>
<rect x="20" y="260" width="300" height="36" rx="6" class="good"/><circle cx="40" cy="278" r="10" fill="#34d399"/><text x="40" y="282" class="num m">6</text><text x="60" y="283" class="ts">renderer->Render(renderWorld)</text>
<rect x="420" y="30" width="320" height="266" rx="10" class="acc"/>
<text x="580" y="52" class="t m">RenderWorld</text>
<text x="580" y="68" class="s m">step 1 empties every section below</text>
<rect x="436" y="80" width="288" height="46" rx="6" class="node"/><text x="450" y="99" class="ts">camera · light · clear · fog</text><text x="450" y="117" class="sp">filled by 2</text>
<rect x="436" y="132" width="288" height="46" rx="6" class="node"/><text x="450" y="151" class="ts">draws: DrawItem[ ]</text><text x="450" y="169" class="sp">filled by 2 (optional) and 4</text>
<rect x="436" y="184" width="288" height="46" rx="6" class="node"/><text x="450" y="203" class="ts">lines: LineVertex[ ]</text><text x="450" y="221" class="sp">filled by 2 (optional)</text>
<rect x="436" y="236" width="288" height="46" rx="6" class="node"/><text x="450" y="255" class="ts">overlay vertices</text><text x="450" y="273" class="sp">filled by 2, 3 and 5 (drawn in that order)</text>
<path d="M320,278 C370,278 370,310 580,310 L580,298" class="ag" marker-end="url(#d9p)"/>
<text x="370" y="324" class="sg">6 reads the finished world</text>
</svg>
<p class="cap">Figure 9 — Who writes which part of the frame. Overlay quads are drawn in the order they were added, so the HUD lands on top of the game UI.</p>

### Interpolation with `alpha`

The simulation ticks at a fixed 60 Hz; the display runs at whatever rate it runs. A frame usually lands *between*
two simulation steps. Drawing the latest step as-is would make motion stutter, so `ExtractDraws` blends:

<svg class="dg" viewBox="0 0 760 210" xmlns="http://www.w3.org/2000/svg">
<line x1="60" y1="150" x2="720" y2="150" stroke="#4a4a4e" stroke-width="1.5"/>
<line x1="120" y1="142" x2="120" y2="158" stroke="#a3a3a8" stroke-width="2"/><text x="120" y="176" class="ts m">step n−1</text>
<line x1="390" y1="142" x2="390" y2="158" stroke="#a3a3a8" stroke-width="2"/><text x="390" y="176" class="ts m">step n</text>
<line x1="660" y1="142" x2="660" y2="158" stroke="#a3a3a8" stroke-width="2"/><text x="660" y="176" class="ts m">step n+1</text>
<text x="255" y="196" class="s m">16.7 ms</text>
<text x="525" y="196" class="s m">16.7 ms</text>
<line x1="498" y1="40" x2="498" y2="160" stroke="#a78bfa" stroke-width="2" stroke-dasharray="4 3"/>
<text x="498" y="30" class="sp m">frame rendered here</text>
<circle cx="390" cy="96" r="16" fill="#242424" stroke="#a3a3a8" stroke-dasharray="3 3"/>
<text x="390" y="72" class="s m">previous</text>
<circle cx="660" cy="96" r="16" fill="#242424" stroke="#a3a3a8" stroke-dasharray="3 3"/>
<text x="660" y="72" class="s m">current</text>
<circle cx="498" cy="96" r="16" fill="#1d2a24" stroke="#34d399" stroke-width="2"/>
<text x="526" y="100" class="sg">drawn</text>
<line x1="390" y1="124" x2="498" y2="124" stroke="#34d399" stroke-width="1.5"/>
<text x="444" y="138" class="sg m">alpha ≈ 0.4</text>
<text x="80" y="96" class="ts">transform = lerp(previous, current, alpha)</text>
</svg>
<p class="cap">Figure 10 — The entity is drawn 40% of the way from its previous simulated position to its current one.</p>

A `RenderWorld` holds only plain values and handles, never GPU objects. That is what lets the same frame go to
`GpuRenderer`, `HeadlessRenderer` or `Capture` unchanged.

## 4. Render: one frame on the GPU

`GpuRenderer.Render` (`GpuRenderer.nox`):

<svg class="dg" viewBox="0 0 760 230" xmlns="http://www.w3.org/2000/svg">
<defs>
<marker id="d11p" markerWidth="10" markerHeight="8" refX="8" refY="3" orient="auto"><path d="M0,0 L8,3 L0,6 Z" fill="#a78bfa"/></marker>
<marker id="d11r" markerWidth="10" markerHeight="8" refX="8" refY="3" orient="auto"><path d="M0,0 L8,3 L0,6 Z" fill="#fb7185"/></marker>
</defs>
<rect x="10" y="40" width="110" height="70" rx="8" class="node"/><circle cx="26" cy="40" r="10" fill="#a78bfa"/><text x="26" y="44" class="num m">1</text><text x="65" y="70" class="ts m">acquire</text><text x="65" y="86" class="ts m">command buf</text>
<line x1="120" y1="75" x2="134" y2="75" class="ar" marker-end="url(#d11p)"/>
<rect x="136" y="40" width="110" height="70" rx="8" class="node"/><circle cx="152" cy="40" r="10" fill="#a78bfa"/><text x="152" y="44" class="num m">2</text><text x="191" y="70" class="ts m">upload lines</text><text x="191" y="86" class="ts m">+ overlay</text><text x="191" y="102" class="s m">copy pass</text>
<line x1="246" y1="75" x2="260" y2="75" class="ar" marker-end="url(#d11p)"/>
<rect x="262" y="40" width="110" height="70" rx="8" class="node"/><circle cx="278" cy="40" r="10" fill="#a78bfa"/><text x="278" y="44" class="num m">3</text><text x="317" y="70" class="ts m">acquire</text><text x="317" y="86" class="ts m">swapchain img</text>
<line x1="372" y1="75" x2="386" y2="75" class="ar" marker-end="url(#d11p)"/>
<rect x="388" y="40" width="110" height="70" rx="8" class="node"/><circle cx="404" cy="40" r="10" fill="#a78bfa"/><text x="404" y="44" class="num m">4</text><text x="443" y="70" class="ts m">depth buffer</text><text x="443" y="86" class="ts m">matches size</text><text x="443" y="102" class="s m">resize → recreate</text>
<line x1="498" y1="75" x2="512" y2="75" class="ar" marker-end="url(#d11p)"/>
<rect x="514" y="40" width="110" height="70" rx="8" class="acc"/><circle cx="530" cy="40" r="10" fill="#a78bfa"/><text x="530" y="44" class="num m">5</text><text x="569" y="70" class="ts m">RecordFrame</text><text x="569" y="86" class="s m">cull, sort, draw</text>
<line x1="624" y1="75" x2="638" y2="75" class="ar" marker-end="url(#d11p)"/>
<rect x="640" y="40" width="110" height="70" rx="8" class="good"/><circle cx="656" cy="40" r="10" fill="#34d399"/><text x="656" y="44" class="num m">6</text><text x="695" y="70" class="ts m">submit</text><text x="695" y="86" class="sg m">SDL presents</text><text x="695" y="102" class="sg m">when GPU done</text>
<path d="M317,110 L317,160" class="ax" marker-end="url(#d11r)"/>
<text x="327" y="140" class="sr">no image (minimised)</text>
<rect x="222" y="162" width="300" height="46" rx="8" class="bad"/>
<text x="372" y="182" class="ts m">submit the empty command buffer</text>
<text x="372" y="199" class="sr m">report presented: false</text>
<text x="20" y="18" class="h">EVERYTHING BEFORE SUBMIT IS ONLY RECORDED ON THE CPU</text>
</svg>
<p class="cap">Figure 11 — Render(), with the early exit when there is nothing to draw into.</p>

1. **Acquire a command buffer.** GPU work is recorded into a command buffer on the CPU and submitted all at once.
2. **Upload this frame's dynamic geometry** (`UploadDynamicGeometry`). Lines and overlay vertices change every frame,
   so they're copied into two growable vertex buffers in a copy pass. The buffers only grow, so a steady frame does
   no allocation.
3. **Acquire the swapchain image** to draw into. If there is none (the window is minimised), submit the empty
   command buffer and report `presented: false`.
4. **Make sure the depth buffer matches** the image's size; it's recreated when the window is resized
   (`EnsureDepthTarget`).
5. **Record the frame** (`RecordFrame`, below).
6. **Submit.** SDL presents the image once the GPU has finished with it.

### RecordFrame

1. **Build the camera matrices.** The aspect ratio comes from the target's size. `Camera.ViewProjection` multiplies
   the projection (perspective, or orthographic for the editor's straight views) by the view (a right-handed
   look-to matrix). Lantern's axes: +Y up, the camera looks down -Z, and clip-space depth runs 0 to 1.

<svg class="dg" viewBox="0 0 760 240" xmlns="http://www.w3.org/2000/svg">
<defs>
<marker id="d12p" markerWidth="10" markerHeight="8" refX="8" refY="3" orient="auto"><path d="M0,0 L8,3 L0,6 Z" fill="#a78bfa"/></marker>
<marker id="d12g" markerWidth="10" markerHeight="8" refX="8" refY="3" orient="auto"><path d="M0,0 L8,3 L0,6 Z" fill="#34d399"/></marker>
<marker id="d12y" markerWidth="10" markerHeight="8" refX="8" refY="3" orient="auto"><path d="M0,0 L8,3 L0,6 Z" fill="#fbbf24"/></marker>
</defs>
<text x="130" y="24" class="h m">WORLD AXES</text>
<line x1="130" y1="160" x2="130" y2="60" stroke="#34d399" stroke-width="2" marker-end="url(#d12g)"/><text x="130" y="52" class="sg m">+Y up</text>
<line x1="130" y1="160" x2="230" y2="160" stroke="#a78bfa" stroke-width="2" marker-end="url(#d12p)"/><text x="244" y="164" class="sp">+X</text>
<line x1="130" y1="160" x2="70" y2="200" stroke="#fbbf24" stroke-width="2" marker-end="url(#d12y)"/><text x="52" y="216" class="s">+Z (toward viewer)</text>
<text x="130" y="232" class="s m">right-handed</text>
<text x="400" y="24" class="h m">CAMERA</text>
<rect x="340" y="110" width="40" height="30" rx="4" class="node"/>
<polygon points="380,116 396,108 396,142 380,134" fill="#242424" stroke="#4a4a4e"/>
<line x1="396" y1="125" x2="470" y2="125" stroke="#a78bfa" stroke-width="2" marker-end="url(#d12p)"/>
<text x="434" y="116" class="sp m">looks down −Z</text>
<text x="400" y="170" class="s m">view matrix = look-to</text>
<text x="400" y="186" class="s m">(position, forward, up)</text>
<text x="640" y="24" class="h m">CLIP-SPACE DEPTH</text>
<rect x="560" y="60" width="160" height="22" fill="#ffffff"/>
<rect x="560" y="82" width="160" height="22" fill="#9a9a9a"/>
<rect x="560" y="104" width="160" height="22" fill="#4d4d4d"/>
<rect x="560" y="126" width="160" height="22" fill="#000000" stroke="#3a3a3a"/>
<text x="550" y="75" class="ts" text-anchor="end">0</text><text x="728" y="75" class="s">near</text>
<text x="550" y="141" class="ts" text-anchor="end">1</text><text x="728" y="141" class="s">far</text>
<text x="640" y="176" class="s m">depth cleared to 1.0 (far)</text>
<text x="640" y="192" class="s m">closer fragment = smaller value</text>
</svg>
<p class="cap">Figure 12 — Lantern's conventions. The camera's own position, forward and field of view live in <code>Camera.nox</code>.</p>

2. **Cull** (`CollectVisibleDraws`). The view-projection matrix gives the frustum's six planes (`Frustum.nox`, the
   Gribb and Hartmann method). Each draw's mesh bounds are moved by its transform and tested against them: a box
   wholly outside any plane is skipped and counted in `drawsCulled`.

<svg class="dg" viewBox="0 0 760 330" xmlns="http://www.w3.org/2000/svg">
<polygon points="130,150 700,30 700,310 130,190" fill="#221d33" stroke="#a78bfa" stroke-width="1.5"/>
<line x1="130" y1="150" x2="130" y2="190" stroke="#c4b5fd" stroke-width="3"/>
<line x1="700" y1="30" x2="700" y2="310" stroke="#c4b5fd" stroke-width="3"/>
<circle cx="80" cy="170" r="14" fill="#242424" stroke="#a78bfa" stroke-width="2"/>
<text x="80" y="174" class="sp m">cam</text>
<text x="118" y="140" class="sp" text-anchor="end">near</text>
<text x="708" y="24" class="sp">far</text>
<text x="420" y="84" class="sp m" transform="rotate(-12 420 84)">side plane</text>
<text x="420" y="268" class="sp m" transform="rotate(12 420 268)">side plane</text>
<rect x="360" y="148" width="54" height="40" rx="3" class="good"/>
<text x="387" y="206" class="sg m">inside: drawn</text>
<rect x="520" y="48" width="64" height="40" rx="3" class="good"/>
<text x="552" y="106" class="sg m">straddles: drawn</text>
<rect x="290" y="34" width="50" height="36" rx="3" class="bad" stroke-dasharray="5 3"/>
<text x="315" y="26" class="sr m">outside left: culled</text>
<rect x="712" y="150" width="34" height="34" rx="3" class="bad" stroke-dasharray="5 3"/>
<text x="729" y="200" class="sr m">beyond</text>
<text x="729" y="213" class="sr m">far</text>
<rect x="20" y="250" width="170" height="60" rx="8" class="node"/>
<text x="105" y="272" class="ts m">drawsCulled = 2</text>
<text x="105" y="290" class="s m">top + bottom planes</text>
<text x="105" y="303" class="s m">not visible from above</text>
</svg>
<p class="cap">Figure 13 — The frustum seen from above. A box is culled only when it is wholly outside at least one plane; a box that straddles a plane is kept.</p>

3. **Sort** (`GpuDrawList.nox`). The visible draws are sorted by texture, then mesh, then submission order, so
   consecutive draws share bindings.

<svg class="dg" viewBox="0 0 760 290" xmlns="http://www.w3.org/2000/svg">
<text x="190" y="26" class="h m">SUBMISSION ORDER</text>
<text x="570" y="26" class="h m">SORTED: TEXTURE → MESH → ORDER</text>
<g>
<rect x="40" y="40" width="300" height="30" rx="4" class="node"/><rect x="40" y="40" width="10" height="30" fill="#34d399"/><text x="60" y="60" class="ts">#1 grass · rock</text><text x="330" y="60" class="sr" text-anchor="end">tex mesh</text>
<rect x="40" y="76" width="300" height="30" rx="4" class="node"/><rect x="40" y="76" width="10" height="30" fill="#fbbf24"/><text x="60" y="96" class="ts">#2 brick · wall</text><text x="330" y="96" class="sr" text-anchor="end">tex mesh</text>
<rect x="40" y="112" width="300" height="30" rx="4" class="node"/><rect x="40" y="112" width="10" height="30" fill="#34d399"/><text x="60" y="132" class="ts">#3 grass · tree</text><text x="330" y="132" class="sr" text-anchor="end">tex mesh</text>
<rect x="40" y="148" width="300" height="30" rx="4" class="node"/><rect x="40" y="148" width="10" height="30" fill="#fbbf24"/><text x="60" y="168" class="ts">#4 brick · wall</text><text x="330" y="168" class="sr" text-anchor="end">tex mesh</text>
<rect x="40" y="184" width="300" height="30" rx="4" class="node"/><rect x="40" y="184" width="10" height="30" fill="#34d399"/><text x="60" y="204" class="ts">#5 grass · rock</text><text x="330" y="204" class="sr" text-anchor="end">tex mesh</text>
<rect x="40" y="220" width="300" height="30" rx="4" class="node"/><rect x="40" y="220" width="10" height="30" fill="#34d399"/><text x="60" y="240" class="ts">#6 grass · tree</text><text x="330" y="240" class="sr" text-anchor="end">mesh</text>
<text x="190" y="274" class="sr m">5 texture binds · 6 mesh binds</text>
</g>
<g>
<rect x="420" y="40" width="300" height="30" rx="4" class="node"/><rect x="420" y="40" width="10" height="30" fill="#fbbf24"/><text x="440" y="60" class="ts">#2 brick · wall</text><text x="710" y="60" class="sg" text-anchor="end">tex mesh</text>
<rect x="420" y="76" width="300" height="30" rx="4" class="node"/><rect x="420" y="76" width="10" height="30" fill="#fbbf24"/><text x="440" y="96" class="ts">#4 brick · wall</text><text x="710" y="96" class="s" text-anchor="end">—</text>
<rect x="420" y="112" width="300" height="30" rx="4" class="node"/><rect x="420" y="112" width="10" height="30" fill="#34d399"/><text x="440" y="132" class="ts">#1 grass · rock</text><text x="710" y="132" class="sg" text-anchor="end">tex mesh</text>
<rect x="420" y="148" width="300" height="30" rx="4" class="node"/><rect x="420" y="148" width="10" height="30" fill="#34d399"/><text x="440" y="168" class="ts">#5 grass · rock</text><text x="710" y="168" class="s" text-anchor="end">—</text>
<rect x="420" y="184" width="300" height="30" rx="4" class="node"/><rect x="420" y="184" width="10" height="30" fill="#34d399"/><text x="440" y="204" class="ts">#3 grass · tree</text><text x="710" y="204" class="sg" text-anchor="end">mesh</text>
<rect x="420" y="220" width="300" height="30" rx="4" class="node"/><rect x="420" y="220" width="10" height="30" fill="#34d399"/><text x="440" y="240" class="ts">#6 grass · tree</text><text x="710" y="240" class="s" text-anchor="end">—</text>
<text x="570" y="274" class="sg m">2 texture binds · 3 mesh binds</text>
</g>
</svg>
<p class="cap">Figure 14 — Six draws, before and after sorting. Right-hand labels mark where a bind actually happens. Ties keep submission order (#2 before #4).</p>

4. **Begin one render pass** with the colour image cleared to the world's clear colour and depth cleared to 1.0, the
   far plane.
5. **Draw the meshes** (`DrawMeshes`). Push the per-frame uniforms once: the view-projection matrix and camera
   position for the vertex shader, and the light, ambient and fog for the fragment shader. Then for each draw, bind
   its mesh's buffers and its texture only if they differ from the previous draw's, push the per-draw uniforms (the
   model matrix, its normal matrix and the tint) and issue one indexed draw.
6. **Draw the lines** (`DrawLines`), depth-tested against the meshes but not writing depth.
7. **Draw the overlay** (`DrawOverlay`) in screen pixels, with no depth test, so it sits on top. Every overlay
   vertex samples the font atlas; solid boxes use the atlas's white cell.
8. **End the pass.**

Steps 4 to 8 build the image in layers, inside a single render pass:

<svg class="dg" viewBox="0 0 760 380" xmlns="http://www.w3.org/2000/svg">
<defs>
<marker id="d15p" markerWidth="10" markerHeight="8" refX="8" refY="3" orient="auto"><path d="M0,0 L8,3 L0,6 Z" fill="#a78bfa"/></marker>
</defs>
<polygon points="220,280 520,280 450,350 150,350" fill="#1f2a3a" stroke="#4a4a4e"/>
<polygon points="220,190 520,190 450,260 150,260" fill="#24242499" stroke="#4a4a4e"/>
<polygon points="250,205 290,205 280,240 240,240" fill="#7c6a9e" stroke="#c4b5fd"/>
<polygon points="330,210 400,210 388,245 318,245" fill="#5b8a74" stroke="#8ee7bf"/>
<polygon points="420,200 450,200 438,230 408,230" fill="#9e7c4a" stroke="#fbbf24"/>
<polygon points="220,100 520,100 450,170 150,170" fill="#24242466" stroke="#4a4a4e"/>
<line x1="200" y1="150" x2="440" y2="120" stroke="#34d399" stroke-width="1.5"/>
<line x1="260" y1="160" x2="300" y2="110" stroke="#fb7185" stroke-width="1.5"/>
<line x1="330" y1="155" x2="470" y2="140" stroke="#fbbf24" stroke-width="1.5"/>
<polygon points="220,10 520,10 450,80 150,80" fill="#24242444" stroke="#4a4a4e"/>
<rect x="200" y="20" width="90" height="16" rx="3" fill="#0e0e0e" stroke="#a78bfa"/>
<rect x="203" y="24" width="60" height="8" fill="#34d399"/>
<text x="360" y="58" class="ts">FPS 60</text>
<rect x="160" y="62" width="120" height="14" rx="3" fill="#0e0e0ecc" stroke="#4a4a4e"/>
<text x="560" y="40" class="t">4 · overlay</text><text x="560" y="58" class="s">screen pixels, no depth test,</text><text x="560" y="72" class="s">alpha blended — always on top</text>
<text x="560" y="130" class="t">3 · lines</text><text x="560" y="148" class="s">depth-tested, no depth write,</text><text x="560" y="162" class="s">alpha blended</text>
<text x="560" y="220" class="t">2 · meshes</text><text x="560" y="238" class="s">lit, sorted, depth test + write,</text><text x="560" y="252" class="s">back faces culled</text>
<text x="560" y="310" class="t">1 · clear</text><text x="560" y="328" class="s">colour → clear colour</text><text x="560" y="342" class="s">depth → 1.0</text>
<line x1="80" y1="340" x2="80" y2="30" class="ar" marker-end="url(#d15p)"/>
<text x="70" y="190" class="sp m" transform="rotate(-90 70 190)">recorded in this order</text>
</svg>
<p class="cap">Figure 15 — One render pass, four layers. Each layer is drawn over the one beneath it.</p>

### The uniforms

Uniform blocks are small sets of constants pushed before a draw. `GpuUniforms.nox` lays each one out exactly as the
GLSL declares it (std140, so every field is a whole number of 16-byte rows):

| Stage, slot | Block | Pushed | Holds |
| --- | --- | --- | --- |
| vertex 0 | `MatrixAndVectorUniforms` (80 bytes) | once per frame | view-projection, camera position |
| vertex 1 | `ObjectUniforms` (128 bytes) | per draw | model matrix, normal matrix |
| fragment 0 | `LightUniforms` (96 bytes) | once per frame | direction to the light, light and ambient colour, fog colour, fog range and the sRGB flag, camera position |
| fragment 1 | `VectorUniforms` (16 bytes) | per draw | tint colour |

<svg class="dg" viewBox="0 0 760 290" xmlns="http://www.w3.org/2000/svg">
<text x="101" y="22" class="h m">VERTEX 0 · FRAME</text>
<text x="289" y="22" class="h m">VERTEX 1 · DRAW</text>
<text x="477" y="22" class="h m">FRAGMENT 0 · FRAME</text>
<text x="665" y="22" class="h m">FRAGMENT 1 · DRAW</text>
<text x="101" y="40" class="ts m">MatrixAndVector · 80</text>
<text x="289" y="40" class="ts m">Object · 128</text>
<text x="477" y="40" class="ts m">Light · 96</text>
<text x="665" y="40" class="ts m">Vector · 16</text>
<rect x="16" y="50" width="170" height="88" rx="3" fill="#2a2340" stroke="#a78bfa"/><text x="101" y="98" class="ts m">viewProjection</text><text x="101" y="112" class="s m">mat4 · 4 rows</text>
<rect x="16" y="138" width="170" height="22" rx="3" fill="#242424" stroke="#4a4a4e"/><text x="101" y="153" class="ts m">cameraPosition</text>
<rect x="204" y="50" width="170" height="88" rx="3" fill="#2a2340" stroke="#a78bfa"/><text x="289" y="98" class="ts m">model</text><text x="289" y="112" class="s m">mat4 · 4 rows</text>
<rect x="204" y="138" width="170" height="88" rx="3" fill="#1d2a24" stroke="#34d399"/><text x="289" y="186" class="ts m">normalMatrix</text><text x="289" y="200" class="s m">3x3 padded to 4x4</text>
<rect x="392" y="50" width="170" height="22" rx="3" fill="#242424" stroke="#4a4a4e"/><text x="477" y="65" class="ts m">directionToLight</text>
<rect x="392" y="72" width="170" height="22" rx="3" fill="#2e2717" stroke="#fbbf24"/><text x="477" y="87" class="ts m">lightColour</text>
<rect x="392" y="94" width="170" height="22" rx="3" fill="#2e2717" stroke="#fbbf24"/><text x="477" y="109" class="ts m">ambientColour</text>
<rect x="392" y="116" width="170" height="22" rx="3" fill="#2e1d29" stroke="#f0a3d0"/><text x="477" y="131" class="ts m">fogColour</text>
<rect x="392" y="138" width="170" height="22" rx="3" fill="#2e1d29" stroke="#f0a3d0"/><text x="477" y="153" class="ts m">fogAndOutput</text>
<rect x="392" y="160" width="170" height="22" rx="3" fill="#242424" stroke="#4a4a4e"/><text x="477" y="175" class="ts m">cameraPosition</text>
<rect x="580" y="50" width="170" height="22" rx="3" fill="#2e1d29" stroke="#f0a3d0"/><text x="665" y="65" class="ts m">tint</text>
<text x="665" y="92" class="s m">lit.frag calls this block</text>
<text x="665" y="106" class="s m">MaterialUniforms</text>
<text x="477" y="200" class="s m">fogAndOutput: x start, y end,</text>
<text x="477" y="214" class="s m">w = 1 when the shader encodes sRGB</text>
<line x1="16" y1="250" x2="186" y2="250" class="grid"/>
<rect x="16" y="244" width="42" height="12" fill="#242424" stroke="#4a4a4e"/><rect x="58" y="244" width="43" height="12" fill="#242424" stroke="#4a4a4e"/><rect x="101" y="244" width="42" height="12" fill="#242424" stroke="#4a4a4e"/><rect x="143" y="244" width="43" height="12" fill="#242424" stroke="#4a4a4e"/>
<text x="101" y="274" class="s m">one row = 16 bytes = four floats</text>
<text x="470" y="240" class="s">std140 rounds every field up to whole rows,</text>
<text x="470" y="256" class="s">so the CPU-side struct must pad to match or</text>
<text x="470" y="272" class="s">the shader reads the wrong numbers.</text>
</svg>
<p class="cap">Figure 16 — The four uniform blocks drawn to scale, one row per 16 bytes. Field order matches <code>GpuUniforms.nox</code> and the GLSL blocks.</p>

The normal matrix is the inverse transpose of the model matrix's 3x3 part. Normals moved by the model matrix itself
would tilt off their surface under non-uniform scale.

<svg class="dg" viewBox="0 0 760 240" xmlns="http://www.w3.org/2000/svg">
<defs>
<marker id="d17g" markerWidth="10" markerHeight="8" refX="8" refY="3" orient="auto"><path d="M0,0 L8,3 L0,6 Z" fill="#34d399"/></marker>
<marker id="d17r" markerWidth="10" markerHeight="8" refX="8" refY="3" orient="auto"><path d="M0,0 L8,3 L0,6 Z" fill="#fb7185"/></marker>
</defs>
<text x="115" y="28" class="h m">ORIGINAL</text>
<line x1="60" y1="60" x2="160" y2="160" stroke="#e8e8ea" stroke-width="3"/>
<line x1="110" y1="110" x2="145" y2="75" class="ag" marker-end="url(#d17g)"/>
<polyline points="117,117 124,110 117,103" fill="none" stroke="#8ee7bf"/>
<text x="115" y="196" class="s m">surface + its normal</text>
<text x="380" y="28" class="h m">SCALE X BY 2 · USE MODEL MATRIX</text>
<line x1="280" y1="60" x2="480" y2="160" stroke="#e8e8ea" stroke-width="3"/>
<line x1="380" y1="110" x2="425" y2="88" class="ax" marker-end="url(#d17r)"/>
<text x="380" y="196" class="sr m">normal leans over —</text>
<text x="380" y="212" class="sr m">no longer perpendicular</text>
<text x="640" y="28" class="h m">SCALE X BY 2 · INVERSE TRANSPOSE</text>
<line x1="540" y1="60" x2="740" y2="160" stroke="#e8e8ea" stroke-width="3"/>
<line x1="640" y1="110" x2="662" y2="65" class="ag" marker-end="url(#d17g)"/>
<polyline points="649,114 652,107 645,103" fill="none" stroke="#8ee7bf"/>
<text x="640" y="196" class="sg m">still perpendicular —</text>
<text x="640" y="212" class="sg m">lighting stays correct</text>
</svg>
<p class="cap">Figure 17 — Why the normal matrix exists. Stretching a surface sideways makes it shallower, so its normal must get steeper, not shallower.</p>

### The shaders

`lit.vert` moves each vertex to world space with the model matrix and to clip space with the view-projection matrix.

<svg class="dg" viewBox="0 0 760 120" xmlns="http://www.w3.org/2000/svg">
<defs>
<marker id="d18p" markerWidth="10" markerHeight="8" refX="8" refY="3" orient="auto"><path d="M0,0 L8,3 L0,6 Z" fill="#a78bfa"/></marker>
</defs>
<rect x="20" y="30" width="150" height="56" rx="8" class="node"/><text x="95" y="54" class="t m">object space</text><text x="95" y="72" class="s m">as stored in the mesh</text>
<line x1="170" y1="58" x2="288" y2="58" class="ar" marker-end="url(#d18p)"/><text x="229" y="50" class="sp m">× model</text>
<rect x="290" y="30" width="150" height="56" rx="8" class="node"/><text x="365" y="54" class="t m">world space</text><text x="365" y="72" class="s m">lighting + fog done here</text>
<line x1="440" y1="58" x2="568" y2="58" class="ar" marker-end="url(#d18p)"/><text x="504" y="50" class="sp m">× viewProjection</text>
<rect x="570" y="30" width="170" height="56" rx="8" class="good"/><text x="655" y="54" class="t m">clip space</text><text x="655" y="72" class="sg m">GPU rasterises from here</text>
</svg>
<p class="cap">Figure 18 — The two matrix multiplies in <code>lit.vert</code>.</p>

`lit.frag` computes:

```text
albedo  = texture(uv) * vertex colour * tint
diffuse = max(dot(normal, directionToLight), 0)
sky     = 0.65 + 0.35 * normal.y                     a cheap hemisphere term: faces pointing up get more ambient
colour  = albedo * (ambient * sky + lightColour * diffuse)
fog     = clamp((distanceToCamera - fogStart) / (fogEnd - fogStart), 0, 1)
colour  = mix(colour, fogColour, fog)                fog fades into the clear colour, hiding the far plane
```

Each line of that has a simple picture:

<svg class="dg" viewBox="0 0 760 270" xmlns="http://www.w3.org/2000/svg">
<defs>
<radialGradient id="d19sph" cx="0.68" cy="0.3" r="0.85"><stop offset="0" stop-color="#e9e2ff"/><stop offset="0.45" stop-color="#8b72d6"/><stop offset="0.75" stop-color="#3a2f5c"/><stop offset="1" stop-color="#241d3a"/></radialGradient>
<linearGradient id="d19fog" x1="0" x2="1"><stop offset="0" stop-color="#8b72d6"/><stop offset="0.3" stop-color="#8b72d6"/><stop offset="0.8" stop-color="#2a3346"/><stop offset="1" stop-color="#2a3346"/></linearGradient>
<marker id="d19y" markerWidth="10" markerHeight="8" refX="8" refY="3" orient="auto"><path d="M0,0 L8,3 L0,6 Z" fill="#fbbf24"/></marker>
<marker id="d19g" markerWidth="10" markerHeight="8" refX="8" refY="3" orient="auto"><path d="M0,0 L8,3 L0,6 Z" fill="#34d399"/></marker>
</defs>
<text x="125" y="24" class="h m">DIFFUSE</text>
<circle cx="115" cy="140" r="70" fill="url(#d19sph)"/>
<line x1="230" y1="40" x2="190" y2="70" stroke="#fbbf24" stroke-width="2" marker-end="url(#d19y)"/>
<text x="232" y="36" class="s" fill="#fbbf24">sun</text>
<line x1="165" y1="90" x2="198" y2="57" class="ag" marker-end="url(#d19g)"/>
<text x="125" y="236" class="s m">brightest where the normal</text>
<text x="125" y="250" class="s m">points at the light; 0 past 90°</text>
<text x="380" y="24" class="h m">SKY (HEMISPHERE)</text>
<line x1="290" y1="190" x2="470" y2="190" class="grid"/>
<line x1="290" y1="190" x2="290" y2="60" class="grid"/>
<line x1="290" y1="152" x2="470" y2="70" stroke="#a78bfa" stroke-width="2.5"/>
<circle cx="290" cy="152" r="4" fill="#a78bfa"/><circle cx="380" cy="111" r="4" fill="#a78bfa"/><circle cx="470" cy="70" r="4" fill="#a78bfa"/>
<text x="296" y="170" class="ts">0.30</text><text x="380" y="130" class="ts m">0.65</text><text x="462" y="62" class="ts" text-anchor="end">1.00</text>
<text x="290" y="206" class="s m">−1</text><text x="380" y="206" class="s m">0</text><text x="470" y="206" class="s m">+1</text>
<text x="380" y="224" class="s m">normal.y</text>
<text x="380" y="242" class="s m">floors get full ambient,</text>
<text x="380" y="256" class="s m">ceilings get 30%</text>
<text x="630" y="24" class="h m">FOG</text>
<line x1="540" y1="150" x2="730" y2="150" class="grid"/>
<line x1="540" y1="150" x2="540" y2="50" class="grid"/>
<polyline points="540,150 597,150 692,60 730,60" fill="none" stroke="#a78bfa" stroke-width="2.5"/>
<line x1="597" y1="145" x2="597" y2="160" class="grid"/><text x="597" y="172" class="s m">fogStart · 60</text>
<line x1="692" y1="145" x2="692" y2="160" class="grid"/><text x="692" y="172" class="s m">fogEnd · 160</text>
<text x="532" y="64" class="s" text-anchor="end">1</text><text x="532" y="154" class="s" text-anchor="end">0</text>
<rect x="540" y="190" width="190" height="22" rx="3" fill="url(#d19fog)"/>
<text x="545" y="230" class="s">scene colour</text><text x="730" y="230" class="s" text-anchor="end">clear colour</text>
<text x="635" y="252" class="s m">distance to camera →</text>
</svg>
<p class="cap">Figure 19 — Diffuse, the hemisphere ambient term, and linear fog. The renderer always fills the fog colour with the world's clear colour, so distant surfaces fade into the background before the far plane cuts them off. Defaults: fog from 60 to 160 units.</p>

`lit.frag` also encodes to sRGB itself when the swapchain can't. `lines` passes colours through. `overlay.vert` turns
pixel positions into clip space (`x / width * 2 - 1`, `1 - y / height * 2`).

<svg class="dg" viewBox="0 0 760 240" xmlns="http://www.w3.org/2000/svg">
<defs>
<marker id="d20p" markerWidth="10" markerHeight="8" refX="8" refY="3" orient="auto"><path d="M0,0 L8,3 L0,6 Z" fill="#a78bfa"/></marker>
<marker id="d20g" markerWidth="10" markerHeight="8" refX="8" refY="3" orient="auto"><path d="M0,0 L8,3 L0,6 Z" fill="#a3a3a8"/></marker>
</defs>
<text x="160" y="22" class="h m">SCREEN PIXELS</text>
<rect x="40" y="40" width="240" height="150" fill="#242424" stroke="#4a4a4e"/>
<text x="44" y="54" class="ts">(0, 0)</text>
<text x="276" y="184" class="ts" text-anchor="end">(width, height)</text>
<line x1="40" y1="40" x2="100" y2="40" stroke="#a3a3a8" stroke-width="2" marker-end="url(#d20g)"/>
<line x1="40" y1="40" x2="40" y2="100" stroke="#a3a3a8" stroke-width="2" marker-end="url(#d20g)"/>
<text x="48" y="112" class="s">+y down</text>
<circle cx="100" cy="77.5" r="6" fill="#34d399"/>
<text x="112" y="82" class="sg">(¼w, ¼h)</text>
<text x="600" y="22" class="h m">CLIP SPACE</text>
<rect x="480" y="40" width="240" height="150" fill="#242424" stroke="#4a4a4e"/>
<line x1="480" y1="115" x2="720" y2="115" class="grid"/>
<line x1="600" y1="40" x2="600" y2="190" class="grid"/>
<text x="484" y="54" class="ts">(−1, 1)</text>
<text x="716" y="184" class="ts" text-anchor="end">(1, −1)</text>
<text x="604" y="128" class="s">(0,0)</text>
<line x1="600" y1="115" x2="600" y2="64" stroke="#a3a3a8" stroke-width="2" marker-end="url(#d20g)"/>
<text x="608" y="70" class="s">+y up</text>
<circle cx="540" cy="77.5" r="6" fill="#34d399"/>
<text x="540" y="100" class="sg m">(−0.5, 0.5)</text>
<line x1="290" y1="115" x2="470" y2="115" class="ar" marker-end="url(#d20p)"/>
<text x="380" y="100" class="ts m">x / w · 2 − 1</text>
<text x="380" y="138" class="ts m">1 − y / h · 2</text>
<text x="380" y="220" class="s m">The flip on y is because pixel rows count down from the top while clip space counts up from the middle.</text>
</svg>
<p class="cap">Figure 20 — The overlay's pixel-to-clip conversion, with one point traced through it.</p>

## 5. Capture and off-screen rendering

`Capture(world, width, height)` records the same frame into a texture of its own instead of the swapchain, copies
the texture into a download transfer buffer, waits on a fence for the GPU to finish, and copies the pixels back into
a `TextureData`. On drivers whose images are blue-green-red, it swaps red and blue. Screenshots
(`LANTERN_CAPTURE_PATH`) and the GPU tests use it. `RenderOffscreen` is the same with a target kept between frames,
for the editor's viewport.

<svg class="dg" viewBox="0 0 760 200" xmlns="http://www.w3.org/2000/svg">
<defs>
<marker id="d21p" markerWidth="10" markerHeight="8" refX="8" refY="3" orient="auto"><path d="M0,0 L8,3 L0,6 Z" fill="#a78bfa"/></marker>
</defs>
<rect x="10" y="40" width="130" height="66" rx="8" class="acc"/><text x="75" y="64" class="ts m">RecordFrame</text><text x="75" y="80" class="s m">into its own</text><text x="75" y="94" class="s m">texture</text>
<line x1="140" y1="73" x2="160" y2="73" class="ar" marker-end="url(#d21p)"/>
<rect x="162" y="40" width="130" height="66" rx="8" class="node"/><text x="227" y="64" class="ts m">copy to</text><text x="227" y="80" class="s m">download</text><text x="227" y="94" class="s m">transfer buffer</text>
<line x1="292" y1="73" x2="312" y2="73" class="ar" marker-end="url(#d21p)"/>
<rect x="314" y="40" width="130" height="66" rx="8" class="node"/><text x="379" y="64" class="ts m">wait on fence</text><text x="379" y="80" class="s m">CPU blocks until</text><text x="379" y="94" class="s m">GPU finishes</text>
<line x1="444" y1="73" x2="464" y2="73" class="ar" marker-end="url(#d21p)"/>
<rect x="466" y="40" width="130" height="66" rx="8" class="node"/><text x="531" y="64" class="ts m">BGRA?</text><text x="531" y="80" class="s m">swap red and</text><text x="531" y="94" class="s m">blue if so</text>
<line x1="596" y1="73" x2="616" y2="73" class="ar" marker-end="url(#d21p)"/>
<rect x="618" y="40" width="130" height="66" rx="8" class="good"/><text x="683" y="64" class="ts m">TextureData</text><text x="683" y="80" class="sg m">RGBA8 in</text><text x="683" y="94" class="sg m">CPU memory</text>
<rect x="490" y="132" width="120" height="40" rx="6" class="node"/><text x="550" y="156" class="ts m">PNG screenshot</text>
<rect x="626" y="132" width="120" height="40" rx="6" class="node"/><text x="686" y="156" class="ts m">GPU test asserts</text>
<path d="M683,106 L683,120 L550,120 L550,130" class="aw"/>
<line x1="683" y1="106" x2="683" y2="130" class="aw"/>
<text x="20" y="150" class="s">RenderOffscreen: same first step,</text>
<text x="20" y="166" class="s">target kept between frames,</text>
<text x="20" y="182" class="s">shown in the editor's viewport.</text>
</svg>
<p class="cap">Figure 21 — Getting a frame back from the GPU. The fence wait is what makes this slow, and why games don't do it every frame.</p>

## 6. Shutdown

`ReleaseDevice` waits for the GPU to go idle, then releases every mesh, texture, buffer, sampler and pipeline, gives
the window back and destroys the device. It must run before the window is destroyed and before SDL shuts down, which
is why the application deletes the game (which owns the renderer) before itself (see
`application-and-game-loop.md`).

<svg class="dg" viewBox="0 0 760 290" xmlns="http://www.w3.org/2000/svg">
<text x="20" y="24" class="h">LIFETIMES · TIME →</text>
<rect x="40" y="40" width="680" height="26" rx="5" fill="#242424" stroke="#4a4a4e"/><text x="52" y="58" class="ts">SDL initialised</text>
<rect x="80" y="76" width="580" height="26" rx="5" fill="#242424" stroke="#4a4a4e"/><text x="92" y="94" class="ts">window</text>
<rect x="120" y="112" width="460" height="26" rx="5" fill="#221d33" stroke="#a78bfa"/><text x="132" y="130" class="ts">game (owns GpuRenderer)</text>
<rect x="140" y="148" width="420" height="26" rx="5" fill="#1d2a24" stroke="#34d399"/><text x="152" y="166" class="ts">GPU device + resources</text>
<line x1="560" y1="34" x2="560" y2="190" stroke="#34d399" stroke-dasharray="3 3"/>
<line x1="580" y1="34" x2="580" y2="190" stroke="#a78bfa" stroke-dasharray="3 3"/>
<line x1="660" y1="34" x2="660" y2="190" stroke="#a3a3a8" stroke-dasharray="3 3"/>
<line x1="720" y1="34" x2="720" y2="190" stroke="#a3a3a8" stroke-dasharray="3 3"/>
<circle cx="560" cy="200" r="9" fill="#34d399"/><text x="560" y="204" class="num m">1</text>
<circle cx="584" cy="200" r="9" fill="#a78bfa"/><text x="584" y="204" class="num m">2</text>
<circle cx="660" cy="200" r="9" fill="#a3a3a8"/><text x="660" y="204" class="num m">3</text>
<circle cx="720" cy="200" r="9" fill="#a3a3a8"/><text x="720" y="204" class="num m">4</text>
<rect x="40" y="222" width="680" height="56" rx="8" class="node"/>
<text x="56" y="242" class="sg">1 ReleaseDevice: wait idle → release meshes, textures, buffers, samplers, pipelines → give window back → destroy device</text>
<text x="56" y="258" class="sp">2 game deleted</text>
<text x="200" y="258" class="s">3 window destroyed</text>
<text x="360" y="258" class="s">4 SDL shuts down</text>
<text x="56" y="272" class="sr">Any other order releases GPU objects against a device or window that no longer exists.</text>
</svg>
<p class="cap">Figure 22 — Each lifetime nests inside the one above it, so teardown runs innermost first.</p>

## What this renderer leaves out, and where it would go

The renderer is small because it does one forward pass with one light, and SDL GPU handles the device, swapchain,
memory and synchronisation underneath. These are the usual next features, and where each would fit:

<svg class="dg" viewBox="0 0 760 370" xmlns="http://www.w3.org/2000/svg">
<defs>
<marker id="d23p" markerWidth="10" markerHeight="8" refX="8" refY="3" orient="auto"><path d="M0,0 L8,3 L0,6 Z" fill="#a78bfa"/></marker>
<marker id="d23g" markerWidth="10" markerHeight="8" refX="8" refY="3" orient="auto"><path d="M0,0 L8,3 L0,6 Z" fill="#a3a3a8"/></marker>
</defs>
<text x="20" y="150" class="h">TODAY</text>
<rect x="20" y="160" width="120" height="56" rx="8" class="node"/><text x="80" y="184" class="t m">RenderWorld</text><text x="80" y="202" class="s m">one light</text>
<line x1="140" y1="188" x2="158" y2="188" class="aw" marker-end="url(#d23g)"/>
<rect x="160" y="160" width="90" height="56" rx="8" class="node"/><text x="205" y="192" class="t m">cull</text>
<line x1="250" y1="188" x2="268" y2="188" class="aw" marker-end="url(#d23g)"/>
<rect x="270" y="160" width="90" height="56" rx="8" class="node"/><text x="315" y="184" class="t m">sort</text><text x="315" y="202" class="s m">tex · mesh</text>
<line x1="360" y1="188" x2="378" y2="188" class="aw" marker-end="url(#d23g)"/>
<rect x="380" y="160" width="220" height="56" rx="8" class="node"/><text x="490" y="184" class="t m">main pass</text><text x="490" y="202" class="s m">opaque · lines · overlay</text>
<line x1="600" y1="188" x2="638" y2="188" class="aw" marker-end="url(#d23g)"/>
<rect x="640" y="160" width="100" height="56" rx="8" class="node"/><text x="690" y="192" class="t m">swapchain</text>
<rect x="20" y="30" width="190" height="64" rx="8" class="ext"/><text x="115" y="54" class="ts m">More lights · Materials</text><text x="115" y="72" class="s m">new data in RenderWorld,</text><text x="115" y="86" class="s m">material handle on DrawItem</text>
<line x1="80" y1="94" x2="80" y2="158" class="ar" marker-end="url(#d23p)"/>
<rect x="230" y="30" width="130" height="64" rx="8" class="ext"/><text x="295" y="54" class="ts m">Instancing</text><text x="295" y="72" class="s m">one draw per</text><text x="295" y="86" class="s m">sorted group</text>
<line x1="315" y1="94" x2="315" y2="158" class="ar" marker-end="url(#d23p)"/>
<rect x="520" y="30" width="220" height="64" rx="8" class="ext"/><text x="630" y="54" class="ts m">Post-processing</text><text x="630" y="72" class="s m">scene → off-screen texture →</text><text x="630" y="86" class="s m">full-screen triangle</text>
<path d="M630,94 L630,130 L619,130 L619,186" class="ar" marker-end="url(#d23p)"/>
<rect x="250" y="250" width="160" height="56" rx="8" class="ext"/><text x="330" y="274" class="ts m">Shadow pass</text><text x="330" y="292" class="s m">depth from the light</text>
<path d="M410,262 C430,262 430,232 430,218" class="ar" marker-end="url(#d23p)"/>
<text x="436" y="244" class="sp">samples map</text>
<rect x="530" y="250" width="200" height="56" rx="8" class="ext"/><text x="630" y="274" class="ts m">Transparency</text><text x="630" y="292" class="s m">back-to-front, after opaque</text>
<path d="M560,250 L560,218" class="ar" marker-end="url(#d23p)"/>
<path d="M250,328 L250,340 L740,340 L740,328" fill="none" stroke="#a78bfa" stroke-width="1.5" stroke-dasharray="6 4"/>
<text x="495" y="360" class="sp m">render graph: orders the passes and owns their textures once there are several</text>
</svg>
<p class="cap">Figure 23 — Today's frame (grey) and where each missing feature plugs in (dashed purple).</p>

| Feature | What changes |
| --- | --- |
| Shadows from the sun | A depth-only pass from the light before the main pass, into a shadow-map texture; a fourth pipeline; the light's view-projection added to `LightUniforms`; `lit.frag` samples the map. `RecordFrame` grows a second pass. |
| Materials | A `Material` type in `renderer/` (shader, textures, parameters) and a material handle on `DrawItem` instead of one texture and a tint. The draw list sorts by pipeline, then material, then mesh. |
| More lights | An array of point and spot lights in `RenderWorld`, a storage buffer or larger uniform block, a loop in `lit.frag`; past a handful of lights, a clustered or deferred approach. |
| Transparency | A second list in `RenderWorld` for blended draws, sorted back to front and drawn after the opaque meshes with depth writes off. |
| Post-processing | Render the scene into an off-screen colour texture (as `Capture` does), then draw a full-screen triangle that samples it (tone mapping, bloom, anti-aliasing) into the swapchain. |
| Instancing | Group draws that share a mesh and material, put their transforms in a buffer, and issue one draw with an instance count. `GpuDrawList.nox` already sorts them together. |
| A render graph | Once there are several passes, describe each pass's inputs and outputs and let a small scheduler order them and manage their textures, instead of a hand-written `RecordFrame`. |

Each of these lives behind `RenderBackend`: the game keeps filling a `RenderWorld`, and only `renderer/` (new
vocabulary) and the backend (new GPU work) change.
