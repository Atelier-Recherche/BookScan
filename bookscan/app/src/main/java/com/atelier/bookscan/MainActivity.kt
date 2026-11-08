package com.atelier.bookscan

import android.Manifest
import android.content.pm.PackageManager
import android.os.Bundle
import android.util.Log
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.result.contract.ActivityResultContracts
import androidx.camera.core.Camera
import androidx.camera.core.CameraSelector
import androidx.camera.core.ImageAnalysis
import androidx.camera.core.ImageCapture
import androidx.camera.core.ImageCaptureException
import androidx.camera.core.Preview
import androidx.camera.lifecycle.ProcessCameraProvider
import androidx.camera.view.PreviewView
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.core.content.ContextCompat
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.lifecycleScope
import java.io.File
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicReference

class MainActivity : ComponentActivity() {
    private var imageCapture: ImageCapture? = null
    private val cameraRef = AtomicReference<Camera?>(null)
    private var companionClient: CompanionClient? = null
    private val capturing = AtomicBoolean(false)

    private val permissionLauncher =
        registerForActivityResult(ActivityResultContracts.RequestPermission()) { }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        if (ContextCompat.checkSelfPermission(this, Manifest.permission.CAMERA)
            != PackageManager.PERMISSION_GRANTED
        ) {
            permissionLauncher.launch(Manifest.permission.CAMERA)
        }

        setContent {
            MaterialTheme {
                BookscanScreen(
                    onBindCapture = { imageCapture = it },
                    onBindCamera = { cameraRef.set(it) },
                    onTorch = { enabled ->
                        cameraRef.get()?.cameraControl?.enableTorch(enabled)
                    },
                    onConnect = { host, ui ->
                        val client = CompanionClient(
                            scope = lifecycleScope,
                            onStatus = { ui.status = it },
                            onMode = { ui.mode = it },
                            onAutoParams = {
                                ui.params = it
                                ui.engine.updateParams(it)
                            },
                            onCaptureNow = {
                                triggerCapture { ok, msg ->
                                    ui.status = if (ok) "semi ok $msg" else msg
                                    if (ok) ui.pages += 1
                                }
                            },
                        )
                        companionClient = client
                        client.connect(host)
                    },
                    onShoot = { ui ->
                        triggerCapture { ok, msg ->
                            ui.status = if (ok) "shot ok $msg" else msg
                            if (ok) ui.pages += 1
                        }
                    },
                    onAutoShot = { ui ->
                        triggerCapture { ok, msg ->
                            ui.status = if (ok) "auto ok $msg" else msg
                            if (ok) ui.pages += 1
                        }
                    },
                    sendPoint = { state -> companionClient?.sendPointState(state) },
                )
            }
        }
    }

    private fun triggerCapture(done: (Boolean, String) -> Unit) {
        if (!capturing.compareAndSet(false, true)) {
            done(false, "busy")
            return
        }
        val capture = imageCapture
        if (capture == null) {
            capturing.set(false)
            done(false, "camera not ready")
            return
        }
        companionClient?.sendCapturing()
        val file = File(cacheDir, "shot_${System.currentTimeMillis()}.jpg")
        val opts = ImageCapture.OutputFileOptions.Builder(file).build()
        capture.takePicture(
            opts,
            ContextCompat.getMainExecutor(this),
            object : ImageCapture.OnImageSavedCallback {
                override fun onImageSaved(outputFileResults: ImageCapture.OutputFileResults) {
                    try {
                        val bytes = file.readBytes()
                        file.delete()
                        val client = companionClient
                        if (client == null) {
                            capturing.set(false)
                            done(false, "not connected")
                            return
                        }
                        client.uploadJpeg(bytes) { ok, msg ->
                            capturing.set(false)
                            done(ok, msg)
                        }
                    } catch (e: Exception) {
                        capturing.set(false)
                        done(false, e.message ?: "read failed")
                    }
                }

                override fun onError(exception: ImageCaptureException) {
                    capturing.set(false)
                    Log.e("bookscan", "capture failed", exception)
                    done(false, exception.message ?: "capture error")
                }
            },
        )
    }

    override fun onDestroy() {
        companionClient?.disconnect()
        super.onDestroy()
    }
}

class BookscanUiState {
    var host by mutableStateOf("192.168.1.75:7842")
    var status by mutableStateOf("idle")
    var mode by mutableStateOf(CaptureMode.SEMI)
    var point by mutableStateOf(PointState.SEARCHING)
    var pages by mutableIntStateOf(0)
    var params by mutableStateOf(AutoParams())
    var layout by mutableStateOf(PageLayout.DOUBLE)
    var overlayVisible by mutableStateOf(true)
    var torchOn by mutableStateOf(false)
    lateinit var engine: AutoCaptureEngine
}

@Composable
private fun BookscanScreen(
    onBindCapture: (ImageCapture) -> Unit,
    onBindCamera: (Camera) -> Unit,
    onTorch: (Boolean) -> Unit,
    onConnect: (String, BookscanUiState) -> Unit,
    onShoot: (BookscanUiState) -> Unit,
    onAutoShot: (BookscanUiState) -> Unit,
    sendPoint: (PointState) -> Unit,
) {
    val ui = remember { BookscanUiState() }
    ui.engine = remember {
        AutoCaptureEngine(
            params = AutoParams(),
            onPointState = {
                ui.point = it
                sendPoint(it)
            },
            onArmedCapture = {
                if (ui.mode == CaptureMode.AUTO) {
                    onAutoShot(ui)
                }
            },
        )
    }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .background(Color(0xFF121417)),
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(12.dp),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            OutlinedTextField(
                value = ui.host,
                onValueChange = { ui.host = it },
                label = { Text("companion host:port") },
                modifier = Modifier.weight(1f),
                singleLine = true,
            )
            Button(onClick = { onConnect(ui.host, ui) }) {
                Text("Connect")
            }
        }

        Text(
            text = "mode=${ui.mode.toWire()} · ${ui.layout.label()} · pages=${ui.pages} · ${ui.status}",
            color = Color(0xFFE8ECEF),
            modifier = Modifier.padding(horizontal = 12.dp),
        )

        Box(
            modifier = Modifier
                .weight(1f)
                .fillMaxWidth(),
        ) {
            CameraPreview(
                onBindCapture = onBindCapture,
                onBindCamera = onBindCamera,
                onFrame = { proxy ->
                    if (ui.mode == CaptureMode.AUTO) {
                        ui.engine.onPreviewFrame(proxy)
                    }
                },
            )
            DetectionOverlay(
                layout = ui.layout,
                point = ui.point,
                visible = ui.overlayVisible,
            )
            PointIndicator(
                state = ui.point,
                modifier = Modifier
                    .align(Alignment.TopCenter)
                    .padding(24.dp),
            )
            Row(
                modifier = Modifier
                    .align(Alignment.TopEnd)
                    .padding(12.dp),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                SmallToggle(
                    label = if (ui.torchOn) "Lampe ON" else "Lampe",
                    active = ui.torchOn,
                    onClick = {
                        ui.torchOn = !ui.torchOn
                        onTorch(ui.torchOn)
                    },
                )
                SmallToggle(
                    label = if (ui.overlayVisible) "Overlay" else "Overlay off",
                    active = ui.overlayVisible,
                    onClick = { ui.overlayVisible = !ui.overlayVisible },
                )
            }
        }

        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 16.dp, vertical = 8.dp),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            PageLayout.entries.forEach { layout ->
                val selected = ui.layout == layout
                Button(
                    onClick = { ui.layout = layout },
                    colors = ButtonDefaults.buttonColors(
                        containerColor = if (selected) Color(0xFF32CDB1) else Color(0xFF2A3038),
                        contentColor = if (selected) Color.Black else Color.White,
                    ),
                    modifier = Modifier.weight(1f),
                ) {
                    Text(layout.label())
                }
            }
        }

        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp),
            horizontalArrangement = Arrangement.SpaceEvenly,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            listOf(CaptureMode.AUTO, CaptureMode.SEMI, CaptureMode.MANUAL).forEach { m ->
                Button(
                    onClick = {
                        ui.mode = m
                        if (m != CaptureMode.AUTO) ui.engine.reset()
                    },
                ) {
                    Text(m.toWire())
                }
            }
            Button(onClick = { onShoot(ui) }) {
                Text("Shoot")
            }
        }
    }
}

@Composable
private fun SmallToggle(
    label: String,
    active: Boolean,
    onClick: () -> Unit,
) {
    Button(
        onClick = onClick,
        colors = ButtonDefaults.buttonColors(
            containerColor = if (active) Color(0xFF32CDB1) else Color(0xCC2A3038),
            contentColor = if (active) Color.Black else Color.White,
        ),
    ) {
        Text(label)
    }
}

@Composable
private fun PointIndicator(state: PointState, modifier: Modifier = Modifier) {
    val color = when (state) {
        PointState.ARMED -> Color(0xFF32CDB1)
        PointState.STABLE -> Color(0xFFFFC107)
        PointState.SEARCHING -> Color(0xFF9AA3AD)
    }
    Column(modifier = modifier, horizontalAlignment = Alignment.CenterHorizontally) {
        Box(
            modifier = Modifier
                .size(18.dp)
                .background(color, CircleShape),
        )
        Spacer(modifier = Modifier.height(4.dp))
        Text(text = state.toWire(), color = Color.White)
    }
}

@Composable
private fun CameraPreview(
    onBindCapture: (ImageCapture) -> Unit,
    onBindCamera: (Camera) -> Unit,
    onFrame: (androidx.camera.core.ImageProxy) -> Unit,
) {
    val context = LocalContext.current
    val lifecycleOwner = LocalLifecycleOwner.current
    val previewView = remember { PreviewView(context) }

    AndroidView(
        factory = { previewView },
        modifier = Modifier.fillMaxSize(),
    )

    LaunchedEffect(lifecycleOwner) {
        val provider = ProcessCameraProvider.getInstance(context).get()
        val preview = Preview.Builder().build().also {
            it.surfaceProvider = previewView.surfaceProvider
        }
        val capture = ImageCapture.Builder()
            .setCaptureMode(ImageCapture.CAPTURE_MODE_MAXIMIZE_QUALITY)
            .build()
        onBindCapture(capture)

        val analysis = ImageAnalysis.Builder()
            .setBackpressureStrategy(ImageAnalysis.STRATEGY_KEEP_ONLY_LATEST)
            .build()
        analysis.setAnalyzer(ContextCompat.getMainExecutor(context)) { image ->
            try {
                onFrame(image)
            } finally {
                image.close()
            }
        }

        provider.unbindAll()
        val camera = provider.bindToLifecycle(
            lifecycleOwner,
            CameraSelector.DEFAULT_BACK_CAMERA,
            preview,
            capture,
            analysis,
        )
        onBindCamera(camera)
    }
}
