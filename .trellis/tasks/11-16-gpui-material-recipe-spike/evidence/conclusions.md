# GPUI 材质画法结论（bahayonghang/devsweep#5）

这份结论是 `bahayonghang/my-ai-cli-toolkit#52`（GPUI 实施规范）的输入。每一项都在本机跑过，数值来自窗口截图上的像素。

- 画廊：`desktop-gpui/examples/material_gallery.rs`，运行 `just gpui-gallery`（参数见文件头，例如 `just gpui-gallery --dark --blob png`）。
- 环境：Windows 10.0.26300，120 dpi（125%），GPUI 为 zed rev `322cd509`，DirectX 渲染，debug 构建。系统开着透明效果，应用模式是深色。
- 数值：父 issue 的「起点数值」，全部在画廊的 `Tokens` 结构体里。
- 原始测量和截图保存在本地签收分支 `feat/airy-glass-material` 的 `.trellis/tasks/11-16-gpui-material-recipe-spike/evidence/`。下文的 `capture.txt`、`measurements.txt` 和 `shots/` 均相对于这个本地目录。A–F 六项实测也已记录在 [DevSweep #5](https://github.com/bahayonghang/devsweep/issues/5#issuecomment-6932241960)。
- 没有运行的项单独列在文末，不计入「可用」。

## 结论一览

| # | 结论 | 一句话 |
| --- | --- | --- |
| A | 可用，推荐做法 (2) | `img()` 画一段内存里的 SVG `radialGradient`。不加资源文件，剖面与理想值相差不超过一个 8 位台阶。 |
| B | 有条件可用 | 填充、描边、内高光、负扩散柔影都能渲染。柔影有三处与 CSS 不同，直接照抄会把纸压暗。 |
| C | 可用 | 合成是逐通道的 sRGB source-over，与计算值相差不超过 1/255。 |
| D | 有条件可用 | 400 和 700 能分别渲染，650 渲染成 700。`tnum` 可用。 |
| E | 有条件可用 | `Blurred` 能让背板透过客户区。`MicaBackdrop` 透出的是壁纸材质，不是窗口后面的内容。两者都要求合成交换链，都不带着色。 |
| F | 可用 | 同一套画法换成深色数值后成立，B 的三处差异同样存在。 |

## A 色斑

目标剖面：中心 alpha 0.70，线性降到半径 0.7 处为 0（SkillPort 的 `radial-gradient(... 0.70, transparent 70%)`）。

| 做法 | 结论 | 量到的剖面（浅色，x 方向，半径的 0 / 0.2 / 0.4 / 0.6 / 0.7） | 新增资源文件 |
| --- | --- | --- | --- |
| 理想值 | | 0.70 / 0.50 / 0.30 / 0.10 / 0.00 | |
| (1) 大模糊 `BoxShadow` | 有条件可用 | 0.56 / 0.41 / 0.09 / 0.00 / 0.00 | 无 |
| (2) `img()` + SVG | 可用，推荐 | 0.72 / 0.50 / 0.31 / 0.09 / 0.00 | 无（SVG 是运行时拼出的字符串） |
| (3) `img()` + PNG | 可用，备选 | 0.69 / 0.50 / 0.31 / 0.09 / 0.00 | 每个颜色每个主题一张，共 6 张，每张约 5.5 KB |
| (4) 四块 `linear_gradient` | 不可用 | 轴向 0.72 / 0.50 / 0.31 / 0.09 / 0.00，对角 0.72 / 0.41 / 0.12 / 0.00 | 无 |

浅色下一个 8 位台阶相当于 alpha 0.031，深色下 0.040。深色的剖面见 `measurements.txt`，结论相同。

推荐 (2)，理由：

- 形状是真正的椭圆径向衰减，三个方向的剖面都贴着理想值。
- 颜色和 alpha 来自 token。换主题或换颜色不用重新出图，也不用往仓库里加文件。
- 一张 256×256 的 SVG 可以拉成任意宽高比。

(3) 的剖面一样好，代价是颜色烤进了文件：改一个色斑颜色要重新生成 PNG。SVG 路径出问题时用它。

(1) 的问题：衰减是高斯，不是线性。画廊里核心取半径的 0.30、sigma 取平均半径的 0.17，中心只到 0.56，形状是体育场形（两头半圆的矩形）而不是椭圆。它可以当「一团柔光」用，做不到与设计稿同形。

(4) 的问题：两个色标的线性渐变拼不出径向衰减，四块拼出来是菱形，放大对比度后能看到四条接缝。

推荐做法的代码：

```rust
const BLOB_EDGE: f32 = 0.70;

fn blob_svg(tint: Tint) -> Arc<Image> {
    let svg = format!(
        concat!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="256" height="256">"##,
            r##"<defs><radialGradient id="g">"##,
            r##"<stop offset="0" stop-color="#{rgb:06X}" stop-opacity="{alpha}"/>"##,
            r##"<stop offset="{edge}" stop-color="#{rgb:06X}" stop-opacity="0"/>"##,
            r##"</radialGradient></defs>"##,
            r##"<rect width="256" height="256" fill="url(#g)"/></svg>"##,
        ),
        rgb = tint.rgb,
        alpha = tint.alpha,
        edge = BLOB_EDGE,
    );
    Arc::new(Image::from_bytes(ImageFormat::Svg, svg.into_bytes()))
}

// 色斑是一个零尺寸的锚点，锚在中心；底场负责定位和裁剪。
let anchor = div().absolute().size(px(0.0)).left(relative(x)).top(relative(y));
anchor.child(
    img(image.clone())
        .absolute()
        .left(px(-rx))
        .top(px(-ry))
        .w(px(2.0 * rx))
        .h(px(2.0 * ry))
        .object_fit(ObjectFit::Fill),
)
```

底场是 `div().absolute().size_full().overflow_hidden().bg(linear_gradient(160.0, …))`，三个色斑是它的子元素。

注意事项：

- `Arc<Image>` 建一次后留着用（画廊在视图创建时建好）。图片的缓存键是内容哈希，每帧重建会每帧重新算哈希。
- 图片在后台线程解码，第一帧没有色斑。截图或做首帧比较前要等一帧。
- SVG 按固有尺寸的 2 倍栅格化一次（256 → 512 px），之后只做缩放。色斑是低频渐变，放大到 1344×1024 逻辑像素看不出问题；把差值放大 6 倍能看到同心台阶（`shots/a-tiles-light.png` 下半条），正常对比度下看不到。
- 这个版本的 `linear_gradient` 只有两个色标，斜向渐变的方向按盒子宽高比压缩，与 CSS 的角到角定义不同。底场的 160° 渐变在宽窗口上不会与网页逐像素一致。渐变带抖动，同一颜色的像素会有 ±2/255 的起伏。

三项附加记录：

- 新增资源文件：见上表。推荐做法不加文件。
- 窗口从 960 到 1600 逻辑像素：四种做法都在两个宽度各截了一张，并与不画色斑的同尺寸窗口相减（`shots/a-field-960.png`、`shots/a-field-960-diff.png`、`shots/a-field-1600.png`、`shots/a-field-1600-diff.png`）。色斑的中心按百分比定位，半径是固定像素，所以窗口变宽后色斑留在三个角上，中间露出纯渐变：顶边 40%–60% 处，960 宽时比无色斑低 3–7 级，1600 宽时差值为 0。四种做法表现一致，没有错位或裁切。截图是在窗口改变尺寸之后取的；连续拖动边框的过程没有观察。
- 空闲 CPU：1600 宽，静置 3 秒后量 20 秒的进程 CPU 时间。不画色斑 31.3 ms，(1) 46.9 ms，(2) 0.0 ms，(3) 15.6 ms，(4) 31.3 ms。计时粒度是 15.6 ms，五个数都不超过单核的 0.23%，没有可测的差别。GPUI 空闲时不重绘。工作集：不画 88.9 MB，(1) 89.1，(2) 93.3，(3) 90.2，(4) 89.1。

截图：`shots/a-tiles-light.png`、`shots/a-tiles-dark.png`（上半条是原样，下半条是与背景的差值放大 6 倍），以及上面四张底场图。

## B 纸

| 成分 | 结论 | 量到的结果（浅色 / 深色） |
| --- | --- | --- |
| 半透明填充 `bg` | 可用 | `#F6FCFA` / `#172129`，等于计算值 |
| 1px 半透明描边 `border_1` + `border_color` | 可用 | 125% 缩放下正好 1 个物理像素，`#FCFEFD` / `#29323A` |
| 顶部 1px 内高光 `BoxShadow { inset: true }` | 有条件可用 | 能渲染，`#FEFFFE` / `#363E46`。它落在描边那一行上，不在描边下面 |
| 负扩散柔影 `spread_radius` 取负 | 有条件可用 | 能渲染。有下面三处差异 |

柔影与 CSS 的三处差异：

1. `blur_radius` 是高斯的 sigma。`blur_radius: px(10.0)` 量到的 sigma 是 10.02 逻辑像素。CSS 的 `blur 40px` 对应 sigma 20，所以要传 CSS 模糊值的一半。
2. 外阴影画在整个元素的下面，不像 CSS 那样只留在盒子外面。纸是半透明的，阴影会透上来：直接给纸加柔影，小样张中心从 `#F6FCFA` 变成 `#EFF6F5`（深色从 `#172129` 变成 `#151E26`），纸越大越明显。
3. 阴影的未模糊矩形与裁剪区域不相交时，GPUI 整条阴影都不画，不管模糊能伸多远。

解决办法是把柔影画成纸的一个子元素，只留纸底边以下的部分：

```rust
/// CSS `0 <y>px <blur>px <spread>px <colour>`。
fn soft_shadow(color: Tint, y: f32, css_blur: f32, spread: f32) -> BoxShadow {
    BoxShadow {
        color: color.color().into(),
        offset: point(px(0.0), px(y)),
        blur_radius: px(css_blur / 2.0),
        spread_radius: px(spread),
        inset: false,
    }
}

/// CSS `inset 0 1px 0 <colour>`。
fn top_highlight(color: Tint) -> BoxShadow {
    BoxShadow {
        color: color.color().into(),
        offset: point(px(0.0), px(1.0)),
        blur_radius: px(0.0),
        spread_radius: px(0.0),
        inset: true,
    }
}

/// 外阴影落在元素底边以下的那一部分。
fn ground_shadow(shadow: BoxShadow, radius: f32) -> Div {
    let reach = f32::from(shadow.offset.y) + 3.0 * f32::from(shadow.blur_radius);
    let drop = (1.0 - f32::from(shadow.offset.y) - f32::from(shadow.spread_radius)).max(0.0);
    div()
        .absolute()
        .left(px(-reach - 1.0))
        .right(px(-reach - 1.0))
        .top(relative(1.0))
        .mt(px(1.0))
        .h(px(reach))
        .overflow_hidden()
        .child(
            div()
                .absolute()
                .left(px(reach))
                .right(px(reach))
                .top(px(drop - 2.0 * reach))
                .h(px(2.0 * reach))
                .rounded(px(radius))
                .shadow(vec![shadow]),
        )
}

fn paper(t: &Tokens) -> Div {
    div()
        .rounded(px(PAPER_RADIUS))
        .bg(t.paper.color())
        .border_1()
        .border_color(t.paper_edge.color())
        .shadow(vec![top_highlight(t.paper_highlight)])
        .child(ground_shadow(
            soft_shadow(t.paper_shadow, 12.0, 40.0, -16.0),
            PAPER_RADIUS,
        ))
}
```

这样画出来纸内是干净的 `#F6FCFA` / `#172129`，纸下 8、16、24 物理像素处是 `#DDEDE9`、`#E0EFEC`、`#E2F2ED`（深色 `#0D171A`、`#0D181B`、`#0E191D`）。

注意事项：

- 纸不能加 `overflow_hidden`，否则挂在纸外的柔影会被裁掉。纸内需要裁剪时，在纸里再套一层。
- `ground_shadow` 只保留底边以下的阴影。`0 12px 40px -16px` 在 CSS 里两侧的阴影 alpha 不到 0.03，丢掉它看不出差别；扩散值不是负数的阴影不适合这样画。
- 因为差异 3，投影元素比 CSS 的位置低：纸的柔影低 5px，浮层的 `0 24px 64px -24px` 低 1px。
- 外阴影的圆角不随 `spread_radius` 调整。
- 内高光与上描边重合成一行：浅色 `#FEFFFE`（描边加高光）对 `#FCFEFD`（只有描边），深色 `#363E46` 对 `#29323A`。要让高光落在描边下面，得另画一个 1px 高的子元素。

截图：`shots/bc-specimens-light.png`、`shots/bc-specimens-dark.png`。从左到右是：填充、加描边、加内高光、直接加柔影（纸被压暗）、只有柔影、柔影只留纸外（推荐画法）、模糊标尺、C 的三个内容块。

## C 内容块

结论：可用。GPUI 的合成是逐通道的 sRGB source-over，与 CSS 相同。

| 块 | 浅色计算值 | 浅色量到 | 深色计算值 | 深色量到 |
| --- | --- | --- | --- | --- |
| 纸（在底场起点色上） | `#F6FCFA` | `#F6FCFA` | `#172129` | `#172129` |
| 输入语义色 12% | `#DBECE6` | `#DBECE6` | `#24373A` | `#25373B` |
| 输出语义色 12% | `#E6E7F6` | `#E5E6F6` | `#2C3343` | `#2C3343` |
| 中性块（ink 5%） | `#ECF2F0` | `#EBF2F0` | `#212B33` | `#222B33` |

```rust
/// 逐通道 sRGB source-over，四舍五入到 8 位。
fn over(top: Tint, under: u32) -> u32 {
    let mix = |shift: u32| {
        let above = f64::from((top.rgb >> shift) & 0xFF);
        let below = f64::from((under >> shift) & 0xFF);
        (top.alpha * above + (1.0 - top.alpha) * below + 0.5).floor() as u32
    };
    (mix(16) << 16) | (mix(8) << 8) | mix(0)
}

const SEMANTIC_FILL_ALPHA: f64 = 0.12;

fn block(fill: Tint) -> Div {
    div().rounded(px(WELL_RADIUS)).bg(fill.color())
}
// block(Tint::new(t.input, SEMANTIC_FILL_ALPHA))
```

注意事项：

- 量到的值与计算值最多差 1/255。计算是先把纸合到底场上取整，再把块合上去；差的这一级应当来自渲染器中间不取整（推断，没有单独验证）。对比度计算用哪一个都不影响结论。
- 块里的文字对比度要按合成后的颜色算，不能按语义色本身算。
- 画廊里的单元测试 `a_semantic_block_on_paper_has_a_predictable_colour` 固定了这组计算值。

截图：`shots/bc-specimens-light.png`、`shots/bc-specimens-dark.png` 最右一块。

## D 字体

结论：有条件可用。

| 字重 | 样张行宽（20px，逻辑像素） | 结果 |
| --- | --- | --- |
| 400 | 274.26 | 单独渲染 |
| 600 | 278.52 | 单独渲染 |
| 650 | 281.32 | 与 700 完全相同 |
| 700 | 281.32 | 单独渲染 |

`tnum`：`111,111.11` 和 `000,000.00` 两串数字，默认是 77.62 和 113.78，打开后都是 111.22。

```rust
const FIGTREE: &[u8] = include_bytes!("material_gallery/Figtree-wght.ttf");

// 启动时
cx.text_system().add_fonts(vec![Cow::Borrowed(FIGTREE)])?;

// 使用
div().font_family("Figtree").font_weight(FontWeight(700.0))

// 等宽数字
fn tabular_figures() -> FontFeatures {
    FontFeatures(Arc::new(vec![("tnum".into(), 1)]))
}
div().font_features(tabular_figures())
```

注意事项：

- 可变字体的字重会落到命名实例上。Figtree 的 `wght` 轴是 300–900，命名实例是整百，650 得到的是 700。规范里的三档字重要写成 400、600、700，或者接受 650 等于 700。
- 注册后的字体族名是 `Figtree`（`all_font_names()` 里能查到）。
- `add_fonts` 失败会返回错误，画廊把它打印出来；产品里要有回退字体。
- 字体文件 `Figtree-wght.ttf` 来自 google/fonts 的 `ofl/figtree`，OFL 许可证放在同一目录。产品要用的话，许可证要进 `desktop-gpui/THIRD_PARTY_NOTICES.txt`。

截图：`shots/d-type-light.png`、`shots/d-type-dark.png`。行宽也打印在 `capture.txt` 的 `stdout` 行里。

## E 窗口材质

结论：有条件可用。只用了公开接口 `WindowOptions::window_background` 和 `Window::set_background_appearance`，没有调用 `vendor/gpui_windows` 的 `try_enable_window_blur`。

取证方法照 `.trellis/spec/backend/desktop-runtime.md`：窗口正后方放一个与窗口同大的无边框窗体，先黑后白，读窗口内同一个屏幕像素。底场 alpha 设为 0（`--field-alpha 0`），所以「底场」探点读到的是窗口材质本身。「纸」探点是工具栏那张纸（白色 0.62）。

| `WindowBackgroundAppearance` | 底场，黑底上 | 底场，白底上 | 纸，黑底上 | 纸，白底上 | 窗口失去焦点后 |
| --- | --- | --- | --- | --- | --- |
| `Opaque` | `#FFFFFF` | `#FFFFFF` | `#FFFFFF` | `#FFFFFF` | 不变 |
| `Transparent` | `#000000` | `#FFFFFF` | `#9E9E9E` | `#FFFFFF` | 不变 |
| `Blurred` | `#090909` | `#D0D0D0` | `#A1A1A1` | `#F0F0F0` | 仍然透出（白底上 `#D3D3D3`） |
| `MicaBackdrop` | `#241D22` | `#241D22` | `#ABA9AB` | `#ABA9AB` | 变成纯色 `#202020` |
| `MicaAltBackdrop` | `#1A0013` | `#1A0013` | `#A89EA6` | `#A89EA6` | 变成纯色 `#202020` |
| `Blurred`，关闭 DirectComposition | `#000000` | `#000000` | `#9E9E9E` | `#9E9E9E` | 不变 |
| `MicaBackdrop`，关闭 DirectComposition | `#000000` | `#000000` | `#9E9E9E` | `#9E9E9E` | 不变 |

怎么读这张表：

- `Blurred`：同一像素在黑底和白底上不同，背板确实透过了客户区，而且是模糊过的（白底上是 `#D0D0D0` 而不是 `#FFFFFF`，因为模糊把窗体外的桌面也卷了进来）。
- `MicaBackdrop`：同一像素在两种背景上相同。Mica 采样的是壁纸，不采样窗口后面的内容，所以按规则的两背景比较看不出透明。它与 `Opaque` 的 `#FFFFFF` 不同，两个探点颜色不同（`#241D22` 和 `#291C1E`），失去焦点后变成 `#202020`，这三点说明系统背板确实显示在客户区里。
- 关闭 DirectComposition（`GPUI_DISABLE_DIRECT_COMPOSITION=1`）后，两种材质都不生效：透明的地方是黑色，不随背景变化。

```rust
// 开窗时
WindowOptions {
    window_background: WindowBackgroundAppearance::Blurred,
    ..Default::default()
}

// 运行中切换
window.set_background_appearance(WindowBackgroundAppearance::MicaBackdrop);

// 底场要让出一部分 alpha，背板才看得见
div().absolute().size_full().overflow_hidden().opacity(0.9).bg(field_gradient(t))
```

注意事项：

- 公开接口不带着色。`Blurred` 的背板本身接近全透明，窗口后面是黑的就透出黑。浅色底场用 alpha 0.9 盖在上面，黑底上的相对亮度是 0.7184 和 0.6992（`e-blurred-a90`），正好压在 `desktop-runtime.md` 要求的 0.70 线上，没有余量。现有产品用 vendored 调用带 `0x99` 着色，量到的是 0.7787 和 0.7273。所以公开的 `Blurred` 不能原样替换 vendored 调用：要么把底场 alpha 提高，要么保留带着色的补丁。
- 公开接口没有返回值，应用不知道材质有没有生效。`desktop-runtime.md` 里「请求成功后才画半透明表面」的规则，换成公开接口后只能靠应用自己判断条件（宽度、高对比度、透明效果开关、是否合成交换链）。关闭 DirectComposition 时不判断就会得到黑色底场。
- Mica 跟随系统的应用模式，不跟随应用内的主题。本机系统是深色，画廊切到浅色主题时 Mica 仍是深色材质；浅色底场 alpha 0.9 盖上去的亮度是 0.7331 和 0.7154。
- Mica 在窗口失去焦点时退回纯色，这是系统行为。
- 纸在 `Transparent` 下是 `#9E9E9E`（255 × 0.62 = 158），说明透明窗口里的合成也是直通 alpha 的 sRGB source-over。
- 父 issue 的材质模型不依赖窗口背板：底场 alpha 为 1 时，上面这些都不影响界面。

截图：`shots/e-active.png`（五种材质，上排黑底，下排白底）、`shots/e-inactive.png`（失去焦点）、`shots/e-variants.png`（底场 alpha 0.9、深色主题、关闭 DirectComposition）。原尺寸：`shots/e-blurred-over-black.png`、`shots/e-blurred-over-white.png`、`shots/e-mica-over-white.png`、`shots/e-mica-over-white-inactive.png`。探点数值在 `capture.txt` 的 `probe` 行。

## F 深色

结论：可用。A–D 的画法不改代码，只换 `Tokens::dark()`，量到的结果在上面各节的深色列里。

- A：四种做法的剖面与浅色同形，(2) 和 (3) 仍然贴着理想值（一个台阶是 alpha 0.040）。
- B：纸 `#172129`，描边 `#29323A`，描边加高光 `#363E46`。三处柔影差异同样存在。
- C：三个块与计算值最多差 1/255。
- 对比度：ink/纸 13.04，muted/纸 6.90，ink/底场 12.86，muted/底场 6.81，控件边界/纸 3.24，与父 issue 的实测表一致。

```rust
const fn dark() -> Self {
    Self {
        field: [0x0F1B1F, 0x18172B],
        blobs: [0x12302A, 0x2A2550, 0x142536],
        blob_alpha: 0.70,
        paper: Tint::new(0x1C2530, 0.60),
        paper_edge: Tint::new(0xFFFFFF, 0.08),
        paper_highlight: Tint::new(0xFFFFFF, 0.06),
        paper_shadow: Tint::new(0x000000, 0.45),
        overlay: Tint::new(0x1E2834, 0.84),
        overlay_highlight: Tint::new(0xFFFFFF, 0.08),
        overlay_shadow: Tint::new(0x000000, 0.60),
        control_edge: Tint::new(0xFFFFFF, 0.36),
        ink: 0xE7EDF3,
        muted: 0xA3AFBD,
        // …
    }
}
```

注意事项：

- 深色底场上柔影很弱：纸底边正下方是 `#0C1518`，底场是 `#0F1B1F`，每个通道只低 3–7 级（浅色是 `#DAEAE7` 对 `#E7F6F1`，低 10–13 级）。深色里纸和底场的区分主要靠填充和描边。
- 直接给纸加柔影在深色里只压暗 2–3 级，比浅色的 7 级轻，但仍建议用 `ground_shadow`，两个主题一套画法。
- 画廊的主题切换是换一份 `Tokens` 再 `cx.notify()`；SVG 色斑每个主题各建一组。

截图：`shots/gallery-dark.png`（整窗），以及各节的 `-dark` 截图。浅色整窗是 `shots/gallery-light.png`。

## 父 issue 实测表里的一处出入

浅色「控件边界在纸上 3.29」只有用 SkillPort 的 ink（`#1F2A37` 0.54）才能复现。按表里写的 `ink`（`#24303A` 0.54）算，最差背景上是 3.17，仍然过 3.0。其余九个数与表一致。画廊的单元测试 `starting_values_reproduce_the_measured_contrast_table` 两个数都断言了。

## 没有运行的项

| 项 | 原因 |
| --- | --- |
| gpui-ce 的 `backdrop_blur` | 未验证。工单规定不换依赖。 |
| macOS、Linux | 本机只有 Windows。 |
| 连续拖动窗口边框时的表现 | 只在改变尺寸后的 960 和 1600 两个宽度截了图。 |
| 关闭系统透明效果、高对比度下的窗口材质 | 需要改 Windows 设置，没有改。 |
| Mica 在受控壁纸上的取色 | 需要换壁纸，没有换。 |
| release 构建下的空闲 CPU | 数值来自 debug 构建。 |
