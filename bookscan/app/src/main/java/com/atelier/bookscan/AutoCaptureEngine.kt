package com.atelier.bookscan

import android.graphics.Bitmap
import android.graphics.ImageFormat
import android.graphics.Rect
import android.graphics.YuvImage
import androidx.camera.core.ImageProxy
import java.io.ByteArrayOutputStream
import kotlin.math.abs

/**
 * Lightweight auto-shutter: motion peak → settle → document-ish frame → armed.
 * Runs on preview frames only (no heavy ML).
 */
class AutoCaptureEngine(
    private var params: AutoParams = AutoParams(),
    private val onPointState: (PointState) -> Unit,
    private val onArmedCapture: () -> Unit,
) {
    private var lastGray: FloatArray? = null
    private var lastW = 0
    private var lastH = 0
    private var sawMotion = false
    private var stableSince = 0L
    private var lastCaptureAt = 0L
    private var lastEmitted = PointState.SEARCHING
    private var lastAnalyzeAt = 0L

    fun updateParams(p: AutoParams) {
        params = p
    }

    fun reset() {
        lastGray = null
        sawMotion = false
        stableSince = 0L
        emit(PointState.SEARCHING)
    }

    fun onPreviewFrame(image: ImageProxy) {
        val now = System.currentTimeMillis()
        if (now - lastAnalyzeAt < 120) { // ~8 fps analysis
            return
        }
        lastAnalyzeAt = now

        val gray = downscaleY(image, 96, 72) ?: return
        val motion = motionScore(gray, lastGray)
        lastGray = gray

        if (now - lastCaptureAt < params.cooldownMs) {
            emit(PointState.SEARCHING)
            return
        }

        if (motion >= params.motionThreshold) {
            sawMotion = true
            stableSince = 0L
            emit(PointState.SEARCHING)
            return
        }

        val hasPage = looksLikePage(gray)
        if (!hasPage) {
            stableSince = 0L
            emit(PointState.SEARCHING)
            return
        }

        if (!sawMotion) {
            // Still allow first shot without prior motion after long stable
            if (stableSince == 0L) stableSince = now
        } else if (stableSince == 0L) {
            stableSince = now
        }

        val stableFor = now - stableSince
        if (stableFor < params.settleMs / 2) {
            emit(PointState.SEARCHING)
        } else if (stableFor < params.settleMs) {
            emit(PointState.STABLE)
        } else {
            emit(PointState.ARMED)
            lastCaptureAt = now
            sawMotion = false
            stableSince = 0L
            onArmedCapture()
        }
    }

    private fun emit(state: PointState) {
        if (state != lastEmitted) {
            lastEmitted = state
            onPointState(state)
        }
    }

    private fun motionScore(cur: FloatArray, prev: FloatArray?): Float {
        if (prev == null || prev.size != cur.size) return 0f
        var sum = 0f
        for (i in cur.indices) {
            sum += abs(cur[i] - prev[i])
        }
        return sum / cur.size
    }

    private fun looksLikePage(gray: FloatArray): Boolean {
        // Bright paper-ish: mean luminance high enough and some edge energy
        var mean = 0f
        for (v in gray) mean += v
        mean /= gray.size
        if (mean < 90f) return false
        var edges = 0
        val w = lastW
        val h = lastH
        if (w < 3 || h < 3) return mean > 120f
        for (y in 1 until h - 1) {
            for (x in 1 until w - 1) {
                val i = y * w + x
                val dx = abs(gray[i - 1] - gray[i + 1])
                val dy = abs(gray[i - w] - gray[i + w])
                if (dx + dy > 40f) edges++
            }
        }
        val edgeRatio = edges.toFloat() / gray.size
        return edgeRatio in 0.02f..0.35f
    }

    private fun downscaleY(image: ImageProxy, tw: Int, th: Int): FloatArray? {
        if (image.format != ImageFormat.YUV_420_888 && image.planes.isEmpty()) return null
        val yPlane = image.planes[0]
        val yBuf = yPlane.buffer
        val rowStride = yPlane.rowStride
        val pixelStride = yPlane.pixelStride
        val w = image.width
        val h = image.height
        lastW = tw
        lastH = th
        val out = FloatArray(tw * th)
        for (ty in 0 until th) {
            val sy = (ty * h) / th
            for (tx in 0 until tw) {
                val sx = (tx * w) / tw
                val index = sy * rowStride + sx * pixelStride
                val v = yBuf.get(index).toInt() and 0xff
                out[ty * tw + tx] = v.toFloat()
            }
        }
        return out
    }

    companion object {
        fun jpegFromYuv(image: ImageProxy, quality: Int = 90): ByteArray? {
            val yBuffer = image.planes[0].buffer
            val uBuffer = image.planes[1].buffer
            val vBuffer = image.planes[2].buffer
            val ySize = yBuffer.remaining()
            val uSize = uBuffer.remaining()
            val vSize = vBuffer.remaining()
            val nv21 = ByteArray(ySize + uSize + vSize)
            yBuffer.get(nv21, 0, ySize)
            vBuffer.get(nv21, ySize, vSize)
            uBuffer.get(nv21, ySize + vSize, uSize)
            val yuv = YuvImage(nv21, ImageFormat.NV21, image.width, image.height, null)
            val out = ByteArrayOutputStream()
            if (!yuv.compressToJpeg(Rect(0, 0, image.width, image.height), quality, out)) {
                return null
            }
            return out.toByteArray()
        }

        fun bitmapToJpeg(bitmap: Bitmap, quality: Int = 90): ByteArray {
            val out = ByteArrayOutputStream()
            bitmap.compress(Bitmap.CompressFormat.JPEG, quality, out)
            return out.toByteArray()
        }
    }
}
