# 智能圆角 / Smart corners

选中一个或多个轮廓节点，在画布左下角的信息框中输入「圆角半径」。
正值生成与相邻线段或曲线相切的圆角；负值切出 V 形墨陷；输入 0 或点击
「移除圆角」恢复原来的节点连接。带圆角的源节点显示一个外圈，仍可直接移动。

Select one or more on-curve nodes and enter **Corner radius** in the Info box.
Positive values create tangent circular fillets; negative values cut V-shaped
ink traps. Zero or **Remove corner** restores the source join. A ring marks each
source node with live corner intent. The original nodes and handles remain editable.

多选时一次修改应用到全部选中的轮廓节点，控制柄和锚点不会受影响。
若原有半径不同，信息框显示「半径不同」；可以输入新的共同值，也可以
点击统一半径按钮，使用第一个节点的半径。一次提交对应一次撤销。

A multi-selection edits all selected on-curve nodes in one undo step. Handles
and anchors are excluded. **Mixed radii** indicates different existing values;
enter a shared radius or choose **Use the first radius for all selected nodes**.

圆角按字体单位设置。节点移动后半径数值保持不变；放置或变换组件时，
组件中的圆角随整个计算后的形状一起变换。拖动源节点期间显示临时节点位置，
派生曲线在提交后更新。目前使用数值输入调整半径，没有专用的半径拖动柄。

Radii use font units. Source-node moves and transforms retain the numeric radius;
component placement transforms the already evaluated drawing. During source-node
dragging the derived path updates after commit. Radius editing currently uses the
numeric field; a dedicated radius drag handle is not provided.

## 范围与几何限制 / Limits

- 半径必须是有限数值，绝对值不超过 8192；每个图层最多 1024 个活动圆角。
  Radii must be finite, with magnitude at most 8192, and at most 1024 active corners
  per layer. Magnitudes below 1e-7 units retain their metadata but display the
  source join, allowing interpolation through zero.
- 正半径支持相邻直线、二次曲线和三次曲线之间可求解的相切圆弧；开放轮廓的
  首尾节点没有两个相邻线段，不能设置圆角。
  Positive radii support bounded tangent solutions between adjacent lines,
  quadratics and cubics. Open-contour endpoints cannot carry live corners.
- 墨陷需要闭合轮廓。半径的绝对值决定沿两侧轮廓退让的距离以及墨陷尖端深度；
  墨陷方向依据整个图层的非零填充区域，字腔边缘也按实际墨色方向切入。
  Traps require closed contours; magnitude sets the arc-length setback on each
  neighboring segment and the tip depth into the layer's actual nonzero fill.
- 半径过大、相邻圆角互相挤占线段、重合节点或无法确定切线时，修改会被拒绝，
  原图形保持不变。不会自动缩小半径。复杂自交或远处轮廓的碰撞没有全局求解器。
  Oversized radii, overlapping neighboring trims, coincident nodes and singular
  or unsolved joins fail atomically. Radii are not silently clamped. There is no
  global collision solver for remote contours or complex self-intersections.

## 保存、烘焙与导出 / Save, bake and export

FCF 保存源节点及半径。TTF、OTF、WOFF 和 WOFF2 导出计算后的曲线，当前文档
仍可编辑。UFO、Glyphs、SFD 和 Type 1 等源格式暂不能保存这些元数据，需先烘焙。

FCF preserves source nodes and radii. TTF, OTF, WOFF and WOFF2 evaluate a copy for
export. Editable source writers such as UFO, Glyphs, SFD and Type 1 reject live
corners until they are baked. Export follows the same point-bound hint and layout
preservation restrictions as other outline-changing effects.

打开「图层 → 图层效果」，先预览再点击「烘焙结果」。即使没有其他效果也可以
烘焙圆角；结果成为普通曲线，并可一次撤销恢复源节点和半径。去重叠、简化、
展开描边及整层曲线转换等重建轮廓的操作，也需要先烘焙或移除圆角。

Open **Layers → Layer effects**, preview, then **Bake result**. No additional
effect is needed. Baking replaces the live construction with ordinary curves;
one undo restores its source nodes and radii. Rebuilding path operations such as
overlap removal, simplification, stroke expansion and whole-layer curve conversion
require corners to be baked or removed first.

局部智能组件在计算圆角前插值半径。全局主版插值要求各主版计算后的节点拓扑
兼容；某一主版新增圆角导致节点数不同时，会报告不兼容。

Local smart components interpolate signed radii before evaluating corners.
Global master interpolation requires compatible evaluated topology; adding a
corner only in one master can produce an explicit incompatibility.
