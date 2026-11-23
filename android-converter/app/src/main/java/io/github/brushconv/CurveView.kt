package io.github.brushconv

import android.content.Context
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.Path
import android.view.View
import io.github.brushconv.core.CurvePoint
import kotlin.math.roundToInt

/**
 * Draws one of a brush's curves: the pen's pressure along the bottom, what it drives up the side.
 * The polyline is what the brush file actually holds, not a fitted approximation.
 */
class CurveView(context: Context, private val points: List<CurvePoint>, private val color: Int) : View(context) {
    private val density = resources.displayMetrics.density
    private val grid = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.STROKE
        strokeWidth = density
        this.color = Color.parseColor("#E0E0E0")
    }
    private val frame = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.STROKE
        strokeWidth = density
        this.color = Color.parseColor("#B0B0B0")
    }
    private val line = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.STROKE
        strokeWidth = 2f * density
        strokeCap = Paint.Cap.ROUND
        strokeJoin = Paint.Join.ROUND
        this.color = color
    }
    private val dot = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.FILL
        this.color = color
    }

    init {
        setBackgroundColor(Color.WHITE)
    }

    override fun onMeasure(widthMeasureSpec: Int, heightMeasureSpec: Int) {
        val width = MeasureSpec.getSize(widthMeasureSpec)
        val height = MeasureSpec.getSize(heightMeasureSpec)
        setMeasuredDimension(width, if (height > 0) height else (130 * density).roundToInt())
    }

    override fun onDraw(canvas: Canvas) {
        val pad = 18f * density
        val left = pad
        val top = 10f * density
        val right = width - pad
        val bottom = height - pad
        if (right <= left || bottom <= top) return

        for (step in 1..3) {
            val share = step / 4f
            canvas.drawLine(left, top + (bottom - top) * share, right, top + (bottom - top) * share, grid)
            canvas.drawLine(left + (right - left) * share, top, left + (right - left) * share, bottom, grid)
        }
        canvas.drawRect(left, top, right, bottom, frame)

        val path = Path()
        var first = true
        for (point in points) {
            val x = left + (right - left) * point.x.toFloat()
            val y = bottom - (bottom - top) * point.y.toFloat()
            if (first) {
                path.moveTo(x, y)
                first = false
            } else {
                path.lineTo(x, y)
            }
        }
        canvas.drawPath(path, line)
        for (point in points) {
            val x = left + (right - left) * point.x.toFloat()
            val y = bottom - (bottom - top) * point.y.toFloat()
            canvas.drawCircle(x, y, 3f * density, dot)
        }
    }
}
