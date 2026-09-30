# 图层效果 / Layer effects

在字形编辑器的「图层」面板打开「图层效果」。操作只针对打开对话框时的
文档、字形和源图层。先预览，再应用；若文档在预览后发生变化，必须重新预览。

Open **Layer effects** from the glyph editor's Layers panel. The dialog captures
the current document, glyph and source layer. Preview before applying. A document
change after that preview requires another review.

添加描边、去除重叠或布尔遮罩；可启用、禁用、上下移动或删除每一步。
列表从上往下计算。源轮廓、组件和锚点仍可编辑；提交源轮廓编辑后，
画布、组件、网格缩略图、文本预览与编译输出使用计算后的几何。
拖动时显示源节点的临时位置，效果结果在提交后更新。

Add a stroke, overlap removal or boolean mask. Enable, disable, reorder or remove
each operation. The stack runs from top to bottom. Source contours, components and
anchors remain editable. After a source edit commits, the canvas, components,
grid thumbnails, text preview and compiled output use evaluated geometry. During
a drag, source nodes show their temporary positions; the effect updates on commit.

- **描边 / Stroke:** 椭圆笔尖（宽高相等时为圆形）或旋转矩形笔尖，可调整
  完整宽度、高度、角度、端点、转角和斜接限制。Elliptical pens include circles;
  rectangular pens provide calligraphic expansion. Dimensions are full pen sizes,
  not radii. Caps affect open ends; joins affect source corners. This operation
  uses one pen size throughout the contour.
- **Variable pen:** Add 2–64 pen points with width, height and angle at normalized
  positions along each centerline. Positions run from 0 (start) to 1 (end), and
  interior positions must increase strictly. Linear progression changes at a
  constant rate; smooth progression eases into and out of each point. Angles
  follow the shortest rotation. Each contour uses the same profile over its own
  length. Closed contours require matching start/end sizes and angles. Source
  edits automatically resample the profile against the edited centerline.
  Expansion uses sampled geometry and reports an approximation warning before
  applying or baking. Excessive sampling fails without changing the source.
- **去除重叠 / Remove overlap:** 按非零填充规则去除内部边界，开放轮廓保持开放。
  Removes interior edges using nonzero fill; open contours remain open.
- **布尔遮罩 / Boolean mask:** 并集、交集、减去遮罩或异或。可以输入闭合 SVG
  路径，或按字形名称捕获默认主版，也可选取当前字形的其他图层。
  Captures an independent copy in layer coordinates. Later edits to the captured
  glyph do not alter that copy. Filled booleans require closed paths; place a stroke
  first when starting from a centerline. Invalid source edits under an enabled
  effect are rejected atomically; disable or repair that effect before continuing.

「应用」保留源轮廓与效果栈。「烘焙结果」将最终结果转为普通轮廓，分解参与的
组件并清空效果栈，可一步撤销。锚点与字宽保留。备份图层会保留自己的效果参数。
涉及多边形重新拟合的预览会显示提示。

**Apply** keeps the source and stack. **Bake result** replaces the source drawing
with final contours, decomposes its components and clears the stack in one undo
step. Anchors and advance widths remain unchanged. Backup layers retain their own
parameters. A preview discloses any polygon curve-refitting fallback.

FCF、FCF.gz 与恢复文件保留源轮廓、顺序和参数。TTF、OTF、WOFF、WOFF2 在导出
副本中计算效果；其他可编辑源文件格式目前要求先烘焙或删除效果栈，否则会在写入
之前报错。与源点绑定的 hint 或程序不能盲目复用于结果；导出副本清除变化图层的
旧 hint，已显式编写的 TrueType 程序要求先烘焙并重新检查点编号。

FCF, compressed FCF and recovery files retain sources, order and parameters.
TTF, OTF, WOFF and WOFF2 evaluate an export copy. Other editable source formats
currently require baking or removing all stacks; otherwise export fails before
touching the destination. Geometry changes invalidate point-bound hints. Authored
TrueType programs require baking and reviewing their compiled point bindings.

局部智能组件先插值，再计算相同的效果栈；参与插值的局部源必须拥有完全相同的
参数。全局设计空间预览与导出插值已经计算的主版轮廓，因此结果拓扑必须兼容。

Local smart sources interpolate before evaluating their shared stack, and require
identical stack definitions. Global designspace preview and export interpolate
evaluated master outlines, whose topology must remain compatible.

每层最多 16 个效果（含禁用项）。笔尖尺寸为 0.1–8192 字体单位，宽高比最多 100，
角度为 ±360°，斜接限制为 1–100。遮罩最多 256 KiB、4096 个路径元素；每一步的
输入最多 4096 个元素，输出最多 65535 个元素。几何超过有符号字体坐标范围时
会报错，不会写入部分结果。

Limits: 16 entries per layer, including disabled effects; pen dimensions
0.1–8192 font units, aspect ratio at most 100, angle within ±360°, miter limit
1–100; stored masks at most 256 KiB and 4096 path elements. Each stage accepts at
most 4096 input elements and 65535 output elements. Out-of-range geometry fails
without committing a partial result.
