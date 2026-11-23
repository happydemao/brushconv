package io.github.brushconv

import android.app.Activity
import android.content.Intent
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.net.Uri
import android.os.Bundle
import android.provider.DocumentsContract
import android.provider.OpenableColumns
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.widget.ArrayAdapter
import android.widget.Button
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.Spinner
import android.widget.TextView
import android.widget.Toast
import io.github.brushconv.core.BrushFields
import io.github.brushconv.core.BrushInfo
import io.github.brushconv.core.FormatInfo
import io.github.brushconv.core.Inspection
import io.github.brushconv.core.convert
import io.github.brushconv.core.fields
import io.github.brushconv.core.formats
import io.github.brushconv.core.inspect
import java.io.File
import java.io.InputStream

/**
 * The whole app: pick brush files, look at what came in and what an export keeps, then convert.
 * There is no painting here, and no other feature to get in the way.
 */
class MainActivity : Activity() {
    private data class Source(val name: String, val bytes: ByteArray, val summary: String, val inspection: Inspection?)

    private data class EditorBrush(val source: Source, val brush: BrushInfo, val index: Int)

    private val sources = ArrayList<Source>()
    private lateinit var shell: FrameLayout
    private var formatList: List<FormatInfo> = emptyList()
    private var formatIndex = 0
    private var brushes: List<EditorBrush> = emptyList()
    private var brushIndex = 0
    private var busy = false
    private var statusView: TextView? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        shell = FrameLayout(this)
        setContentView(shell)

        formatList = try {
            formats()
        } catch (error: Throwable) {
            emptyList()
        }
        formatList.indexOfFirst { it.name == "brushset" }.takeIf { it >= 0 }?.let { formatIndex = it }
        showPicker()
    }

    // ---- the picker ---------------------------------------------------------------------

    private fun showPicker() {
        val root = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(20), dp(16), dp(20), dp(12))
        }
        root.addView(
            TextView(this).apply {
                text = "笔刷转换器"
                textSize = 24f
                setTypeface(typeface, Typeface.BOLD)
            },
        )
        root.addView(
            TextView(this).apply {
                text = "只做一件事：把笔刷文件转换成别的格式。支持 .brush、.brushset、.abr、.kpp、.bundle、.gbr、.gih、.vbr、.sut、.sutg、.prbr。"
                textSize = 13f
                setTextColor(Color.DKGRAY)
                setPadding(0, dp(6), 0, dp(14))
            },
        )
        root.addView(
            Button(this).apply {
                text = "① 选择笔刷文件"
                setOnClickListener { pickFiles() }
            },
        )
        if (sources.isNotEmpty()) {
            root.addView(
                Button(this).apply {
                    text = "查看画笔编辑器"
                    setOnClickListener { showEditor() }
                },
                LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT).apply {
                    topMargin = dp(8)
                },
            )
        }
        root.addView(
            TextView(this).apply {
                text = if (sources.isEmpty()) {
                    "还没有选择文件"
                } else {
                    sources.joinToString("\n\n") { "• ${it.name}\n   ${it.summary}" }
                }
                textSize = 13f
                setTextColor(Color.DKGRAY)
                setPadding(dp(10), dp(8), dp(10), dp(8))
            },
            LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT).apply {
                topMargin = dp(10)
            },
        )
        shell.removeAllViews()
        shell.addView(root)
    }

    private fun pickFiles() {
        val intent = Intent(Intent.ACTION_OPEN_DOCUMENT).apply {
            addCategory(Intent.CATEGORY_OPENABLE)
            type = "*/*"
            putExtra(Intent.EXTRA_ALLOW_MULTIPLE, true)
        }
        startActivityForResult(intent, REQUEST_PICK)
    }

    private fun load(uris: List<Uri>) {
        for (uri in uris) {
            val name = displayName(uri)
            try {
                val bytes = contentResolver.openInputStream(uri).use { stream -> readFully(stream) }
                if (bytes == null || bytes.isEmpty()) {
                    toast("✗ $name: 文件是空的")
                    continue
                }
                val inspection: Inspection? = try {
                    inspect(name, bytes)
                } catch (error: Throwable) {
                    null
                }
                val summary = if (inspection == null) {
                    "无法识别的笔刷格式"
                } else {
                    buildString {
                        append(inspection.format)
                        append(" · ")
                        append(inspection.brushes.size)
                        append(" 个笔刷")
                        if (inspection.setName.isNotBlank()) {
                            append(" · ")
                            append(inspection.setName)
                        }
                    }
                }
                sources.add(Source(name, bytes, summary, inspection))
            } catch (error: Throwable) {
                toast("✗ $name: ${error.message ?: error.toString()}")
            }
        }
        brushes = sources.flatMap { source ->
            source.inspection?.brushes?.mapIndexed { index, brush -> EditorBrush(source, brush, index) } ?: emptyList()
        }
        if (brushes.isEmpty()) {
            showPicker()
            toast("没有读到笔刷")
            return
        }
        brushIndex = 0
        showEditor()
    }

    // ---- the brush editor ---------------------------------------------------------------

    private fun showEditor() {
        if (brushes.isEmpty()) {
            showPicker()
            return
        }
        brushIndex = brushIndex.coerceIn(0, brushes.lastIndex)

        val root = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(16), dp(10), dp(16), dp(10))
        }

        root.addView(
            LinearLayout(this).apply {
                orientation = LinearLayout.HORIZONTAL
                gravity = Gravity.CENTER_VERTICAL
                addView(
                    Button(this@MainActivity).apply {
                        text = "← 返回"
                        setOnClickListener { showPicker() }
                    },
                )
                addView(
                    TextView(this@MainActivity).apply {
                        text = "画笔编辑器"
                        textSize = 20f
                        setTypeface(typeface, Typeface.BOLD)
                        setPadding(dp(12), 0, 0, 0)
                    },
                )
            },
        )

        val brushNames = brushes.mapIndexed { index, entry ->
            if (sources.size > 1) {
                "${index + 1}. ${entry.brush.name}  ·  ${entry.source.name}"
            } else {
                "${index + 1}. ${entry.brush.name}"
            }
        }
        val brushSpinner = Spinner(this).apply {
            adapter = ArrayAdapter(this@MainActivity, android.R.layout.simple_spinner_item, brushNames).apply {
                setDropDownViewResource(android.R.layout.simple_spinner_dropdown_item)
            }
            setSelection(brushIndex)
        }
        root.addView(brushSpinner, LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT).apply { topMargin = dp(8) })

        root.addView(
            TextView(this).apply {
                text = "目标格式"
                textSize = 14f
                setTypeface(typeface, Typeface.BOLD)
                setPadding(0, dp(8), 0, dp(2))
            },
        )
        val formatSpinner = Spinner(this).apply {
            adapter = ArrayAdapter(
                this@MainActivity,
                android.R.layout.simple_spinner_item,
                formatList.map { "${it.label}  (${it.name})" },
            ).apply {
                setDropDownViewResource(android.R.layout.simple_spinner_dropdown_item)
            }
            setSelection(formatIndex.coerceIn(0, formatList.lastIndex.coerceAtLeast(0)))
        }
        root.addView(formatSpinner)

        root.addView(
            LinearLayout(this).apply {
                orientation = LinearLayout.HORIZONTAL
                gravity = Gravity.CENTER_VERTICAL
                setPadding(0, dp(8), 0, dp(4))
                addView(dot(IMPORTED_COLOR))
                addView(legend("已导入"))
                addView(dot(EXPORTABLE_COLOR))
                addView(legend("可导出"))
                addView(dot(OFF_COLOR))
                addView(legend("默认 / 不包含"))
            },
        )

        val fieldsView = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL }
        val fieldsScroll = ScrollView(this).apply {
            addView(fieldsView)
            background = GradientDrawable().apply { setColor(0xFFF7F7F7.toInt()); cornerRadius = dp(8).toFloat() }
        }
        root.addView(
            fieldsScroll,
            LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, 0, 1f).apply { topMargin = dp(4) },
        )

        val status = TextView(this).apply {
            textSize = 12f
            setTextColor(Color.DKGRAY)
            setPadding(0, dp(6), 0, 0)
        }
        statusView = status
        root.addView(status)

        root.addView(
            Button(this).apply {
                text = "② 转换到文件夹…"
                isEnabled = !busy && formatList.isNotEmpty()
                setOnClickListener { if (!busy) chooseFolder() }
            },
            LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT).apply {
                topMargin = dp(8)
            },
        )

        shell.removeAllViews()
        shell.addView(root)

        brushSpinner.onItemSelectedListener = SimpleItemListener { position ->
            brushIndex = position
            renderFields(fieldsView, status, brushSpinner.selectedItemPosition, formatSpinner.selectedItemPosition)
        }
        formatSpinner.onItemSelectedListener = SimpleItemListener { position ->
            formatIndex = position
            renderFields(fieldsView, status, brushSpinner.selectedItemPosition, position)
        }
        renderFields(fieldsView, status, brushIndex, formatSpinner.selectedItemPosition)
    }

    private fun renderFields(fieldsView: LinearLayout, status: TextView, brushAt: Int, formatAt: Int) {
        fieldsView.removeAllViews()
        val entry = brushes.getOrNull(brushAt.coerceIn(0, brushes.lastIndex)) ?: return
        val source = entry.source
        val brush = entry.brush
        val format = formatList.getOrNull(formatAt.coerceIn(0, formatList.lastIndex.coerceAtLeast(0)))
        status.text = ""
        val report: List<BrushFields> = try {
            fields(source.name, source.bytes, format?.name ?: "brushset")
        } catch (error: Throwable) {
            status.text = "读不出字段：${error.message ?: error.toString()}"
            return
        }
        val reportEntry = report.getOrNull(entry.index) ?: return
        val importedCount = reportEntry.fields.count { it.imported }
        val exportableCount = reportEntry.fields.count { it.exportable }
        fieldsView.addView(
            TextView(this).apply {
                text = "${brush.name} · 来自 ${source.name}"
                textSize = 14f
                setTypeface(typeface, Typeface.BOLD)
                setPadding(dp(10), dp(8), dp(10), dp(2))
            },
        )
        fieldsView.addView(
            TextView(this).apply {
                text = "${format?.label ?: ""} 会保留 $exportableCount 项；这个文件带入了 $importedCount 项"
                textSize = 12f
                setTextColor(Color.DKGRAY)
                setPadding(dp(10), 0, dp(10), dp(6))
            },
        )
        for (field in reportEntry.fields) {
            fieldsView.addView(
                LinearLayout(this).apply {
                    orientation = LinearLayout.HORIZONTAL
                    gravity = Gravity.CENTER_VERTICAL
                    setPadding(dp(10), dp(5), dp(10), dp(5))
                    addView(if (field.imported) dot(IMPORTED_COLOR) else dot(OFF_COLOR))
                    addView(if (field.exportable) dot(EXPORTABLE_COLOR) else dot(OFF_COLOR))
                    addView(
                        TextView(this@MainActivity).apply {
                            text = labelOf(field.key, field.label)
                            textSize = 14f
                            setPadding(dp(8), 0, dp(8), 0)
                            setTypeface(typeface, if (field.imported || field.exportable) Typeface.BOLD else Typeface.NORMAL)
                        },
                        LinearLayout.LayoutParams(0, ViewGroup.LayoutParams.WRAP_CONTENT, 1f),
                    )
                    addView(
                        TextView(this@MainActivity).apply {
                            text = field.value
                            textSize = 13f
                            setTextColor(if (field.imported) Color.DKGRAY else Color.GRAY)
                        },
                    )
                },
            )
        }
    }

    private fun labelOf(key: String, fallback: String): String = when (key) {
        "name" -> "名称"
        "tip" -> "笔尖"
        "grain" -> "颗粒 / 纹理"
        "spacing" -> "间距"
        "size" -> "尺寸范围"
        "opacity" -> "不透明度"
        "flow" -> "流量"
        "pressure_size" -> "压力 → 尺寸"
        "pressure_opacity" -> "压力 → 不透明度"
        "pressure_flow" -> "压力 → 流量"
        "scatter" -> "散布"
        "jitter" -> "尺寸抖动"
        "rotation" -> "旋转角度"
        "dual" -> "双笔刷"
        "author" -> "作者"
        "thumbnail" -> "缩略图"
        else -> fallback
    }

    private fun dot(color: Int): View = View(this).apply {
        background = GradientDrawable().apply {
            shape = GradientDrawable.OVAL
            setColor(color)
        }
        layoutParams = LinearLayout.LayoutParams(dp(10), dp(10)).apply { marginEnd = dp(3) }
    }

    private fun legend(text: String): TextView = TextView(this).apply {
        this.text = text
        textSize = 11f
        setTextColor(Color.DKGRAY)
        setPadding(0, 0, dp(10), 0)
    }

    // ---- conversion ---------------------------------------------------------------------

    private fun chooseFolder() {
        if (sources.isEmpty()) {
            toast("先选择笔刷文件")
            return
        }
        if (formatList.isEmpty()) {
            toast("没有可用的输出格式")
            return
        }
        startActivityForResult(Intent(Intent.ACTION_OPEN_DOCUMENT_TREE), REQUEST_FOLDER)
    }

    @Deprecated("The platform's own result API keeps the app free of AndroidX")
    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)
        if (resultCode != RESULT_OK || data == null) return
        when (requestCode) {
            REQUEST_PICK -> {
                val uris = ArrayList<Uri>()
                data.data?.let { uris.add(it) }
                data.clipData?.let { clip ->
                    for (index in 0 until clip.itemCount) uris.add(clip.getItemAt(index).uri)
                }
                load(uris)
            }
            REQUEST_FOLDER -> data.data?.let { startConversion(it) }
        }
    }

    private fun startConversion(tree: Uri) {
        val format = formatList.getOrNull(formatIndex) ?: return
        val inputs = ArrayList(sources)
        busy = true
        statusView?.text = "开始转换 ${inputs.size} 个文件 → ${format.label} (${format.name})"
        Thread {
            val outputs = ArrayList<Pair<String, ByteArray>>()
            val cacheRoot = File(cacheDir, "converted")
            cacheRoot.deleteRecursively()
            cacheRoot.mkdirs()
            inputs.forEachIndexed { index, source ->
                val dir = File(cacheRoot, "job-$index")
                dir.mkdirs()
                try {
                    val names = convert(source.name, source.bytes, format.name, null, dir.absolutePath)
                    names.forEach { name -> outputs.add(name to File(dir, name).readBytes()) }
                    post("✓ ${source.name} → ${names.size} 个文件")
                } catch (error: Throwable) {
                    post("✗ ${source.name}: ${error.message ?: error.toString()}")
                }
                dir.deleteRecursively()
            }
            cacheRoot.deleteRecursively()
            runOnUiThread {
                if (outputs.isEmpty()) {
                    post("没有生成任何文件")
                    busy = false
                    return@runOnUiThread
                }
                Thread {
                    val written = copyInto(tree, outputs)
                    runOnUiThread {
                        post("完成：$written 个文件已写入所选文件夹")
                        Toast.makeText(this, "转换完成：$written 个文件", Toast.LENGTH_SHORT).show()
                        busy = false
                    }
                }.start()
            }
        }.start()
    }

    private fun copyInto(tree: Uri, outputs: List<Pair<String, ByteArray>>): Int {
        val resolver = contentResolver
        val taken = HashSet<String>()
        try {
            val children = DocumentsContract.buildChildDocumentsUriUsingTree(tree, DocumentsContract.getTreeDocumentId(tree))
            resolver.query(
                children,
                arrayOf(DocumentsContract.Document.COLUMN_DISPLAY_NAME),
                null,
                null,
                null,
            )?.use { cursor ->
                while (cursor.moveToNext()) {
                    taken.add(cursor.getString(0).lowercase())
                }
            }
        } catch (error: Throwable) {
            post("! 读不到目标文件夹里的文件名：${error.message}")
        }
        var count = 0
        for ((name, bytes) in outputs) {
            val candidate = uniqueName(name, taken)
            try {
                val mime = if (candidate.endsWith(".json", true)) "application/json" else "application/octet-stream"
                val document = DocumentsContract.createDocument(resolver, tree, mime, candidate) ?: continue
                resolver.openOutputStream(document)?.use { it.write(bytes) } ?: continue
                count++
            } catch (error: Throwable) {
                post("✗ 写入 $candidate 失败：${error.message}")
            }
        }
        return count
    }

    private fun uniqueName(name: String, taken: HashSet<String>): String {
        var candidate = name
        var number = 2
        while (!taken.add(candidate.lowercase())) {
            val dot = name.lastIndexOf('.')
            candidate = if (dot > 0) {
                "${name.substring(0, dot)} ($number)${name.substring(dot)}"
            } else {
                "$name ($number)"
            }
            number++
        }
        return candidate
    }

    // ---- small helpers ------------------------------------------------------------------

    private fun post(line: String) {
        runOnUiThread { statusView?.append("\n$line") }
    }

    private fun toast(line: String) {
        Toast.makeText(this, line, Toast.LENGTH_SHORT).show()
    }

    private fun readFully(stream: InputStream?): ByteArray? = stream?.use { it.readBytes() }

    private fun displayName(uri: Uri): String {
        contentResolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use { cursor ->
            if (cursor.moveToFirst()) {
                val index = cursor.getColumnIndex(OpenableColumns.DISPLAY_NAME)
                if (index >= 0) return cursor.getString(index)
            }
        }
        return uri.lastPathSegment ?: "brush"
    }

    private fun dp(value: Int): Int = (value * resources.displayMetrics.density).toInt()

    private class SimpleItemListener(private val action: (Int) -> Unit) : android.widget.AdapterView.OnItemSelectedListener {
        override fun onItemSelected(parent: android.widget.AdapterView<*>?, view: View?, position: Int, id: Long) {
            action(position)
        }

        override fun onNothingSelected(parent: android.widget.AdapterView<*>?) = Unit
    }

    companion object {
        private const val REQUEST_PICK = 1
        private const val REQUEST_FOLDER = 2
        private const val IMPORTED_COLOR = 0xFF2E7D32.toInt()
        private const val EXPORTABLE_COLOR = 0xFF1565C0.toInt()
        private const val OFF_COLOR = 0xFFCfCfCf.toInt()
    }
}
