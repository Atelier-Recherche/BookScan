package com.atelier.bookscan

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.graphics.drawscope.Stroke

@Composable
fun DetectionOverlay(
    layout: PageLayout,
    point: PointState,
    visible: Boolean,
    modifier: Modifier = Modifier,
) {
    if (!visible) return
    val strokeColor = when (point) {
        PointState.ARMED -> Color(0xFF32CDB1)
        PointState.STABLE -> Color(0xFFFFC107)
        PointState.SEARCHING -> Color(0x99FFFFFF)
    }
    val dash = PathEffect.dashPathEffect(floatArrayOf(18f, 12f), 0f)

    Canvas(modifier = modifier.fillMaxSize()) {
        val w = size.width
        val h = size.height
        val marginX = w * 0.08f
        val marginY = h * 0.10f
        val stroke = Stroke(width = 4f, pathEffect = dash)

        when (layout) {
            PageLayout.SINGLE -> {
                drawRoundRect(
                    color = strokeColor,
                    topLeft = Offset(marginX, marginY),
                    size = Size(w - marginX * 2, h - marginY * 2),
                    cornerRadius = CornerRadius(24f, 24f),
                    style = stroke,
                )
            }
            PageLayout.DOUBLE -> {
                val mid = w / 2f
                val gutter = w * 0.02f
                // Left page
                drawRoundRect(
                    color = strokeColor,
                    topLeft = Offset(marginX, marginY),
                    size = Size(mid - marginX - gutter, h - marginY * 2),
                    cornerRadius = CornerRadius(18f, 18f),
                    style = stroke,
                )
                // Right page
                drawRoundRect(
                    color = strokeColor,
                    topLeft = Offset(mid + gutter, marginY),
                    size = Size(mid - marginX - gutter, h - marginY * 2),
                    cornerRadius = CornerRadius(18f, 18f),
                    style = stroke,
                )
                // Gutter / spine
                drawLine(
                    color = strokeColor.copy(alpha = 0.7f),
                    start = Offset(mid, marginY),
                    end = Offset(mid, h - marginY),
                    strokeWidth = 3f,
                )
            }
        }
    }
}
