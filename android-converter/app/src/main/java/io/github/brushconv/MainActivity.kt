package io.github.brushconv

import android.app.Activity
import android.content.Intent
import android.graphics.Color
import android.graphics.Typeface
import android.net.Uri
import android.os.Bundle
import android.provider.DocumentsContract
import android.provider.OpenableColumns
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.widget.ArrayAdapter
import android.widget.Button
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.Spinner
import android.widget.TextView
import android.widget.Toast
import io.github.brushconv.core.FormatInfo
import io.github.brushconv.core.convert
import io.github.brushconv.core.formats
import io.github.brushconv.core.inspect
import io.github.brushconv.core.Inspection
import java.io.File
import java.io.InputStream

/**
 * The whole app: pick brush files, pick a format, pick a folder, convert. There is no painting
 * here, and no other feature to get in the way.
 */
class MainActivity : Activity() {
    private data class Source(val name: String, val bytes: ByteArray, val summary: String)

    private val sources = ArrayList<Source>()
    private lateinit var selectedView: TextView
    private lateinit var logView: TextView
    private lateinit var logScroll: ScrollView
    private lateinit var formatSpinner: Spinner
    private lateinit var convertButton: Button
    private var formatList: List<FormatInfo> = emptyList()
    private var busy = false

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

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

        selectedView = TextView(this).apply {
            text = "还没有选择文件"
            textSize = 13f
            setTextColor(Color.DKGRAY)
            setPadding(dp(10), dp(8), dp(10), dp(8))
        }
        root.addView(
            ScrollView(this).apply {
                background = android.graphics.drawable.ColorDrawable(0xFFF2F2F2.toInt())
                addView(selectedView)
                layoutParams = LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, dp(120)).apply {
                    topMargin = dp(8)
                    bottomMargin = dp(14)
                }
            },
        )

        root.addView(
            TextView(this).apply {
                text = "输出格式"
                textSize = 14f
                setTypeface(typeface, Typeface.BOLD)
            },
        )

        formatSpinner = Spinner(this)
        root.addView(formatSpinner)

        convertButton = Button(this).apply {
            text = "② 转换到文件夹…"
            isEnabled = false
            setOnClickListener { if (!busy) chooseFolder() }
        }
        root.addView(
            convertButton,
            LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT).apply {
                topMargin = dp(12)
            },
        )

        logView = TextView(this).apply {
            textSize = 12f
            setTextColor(Color.DKGRAY)
            typeface = Typeface.MONOSPACE
            setPadding(dp(10), dp(8), dp(10), dp(8))
        }
        logScroll = ScrollView(this).apply {
            background = android.graphics.drawable.ColorDrawable(0xFFFAFAFA.toInt())
            addView(logView)
            layoutParams = LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, 0, 1f).apply {
                topMargin = dp(14)
            }
        }
        root.addView(logScroll)

        setContentView(root)

        formatList = try {
            formats()
        } catch (error: Throwable) {
            log("✗ 转换器没有加载：${error.message}")
            emptyList()
        }
        val labels = formatList.map { "${it.label}  (${it.name})" }
        formatSpinner.adapter = ArrayAdapter(this, android.R.layout.simple_spinner_item, labels).apply {
            setDropDownViewResource(android.R.layout.simple_spinner_dropdown_item)
        }
        if (formatList.isNotEmpty()) formatSpinner.setSelection(0)

        log("提示：格式说明里的“.kpp / .gbr / .brush”会为每个笔刷写一个文件，其余格式写一个文件。")
    }

    private fun dp(value: Int): Int = (value * resources.displayMetrics.density).toInt()

    private fun log(line: String) {
        runOnUiThread {
            if (logView.text.isNotEmpty()) logView.append("\n")
            logView.append(line)
            logScroll.post { logScroll.fullScroll(View.FOCUS_DOWN) }
        }
    }

    private fun pickFiles() {
        val intent = Intent(Intent.ACTION_OPEN_DOCUMENT).apply {
            addCategory(Intent.CATEGORY_OPENABLE)
            type = "*/*"
            putExtra(Intent.EXTRA_ALLOW_MULTIPLE, true)
        }
        startActivityForResult(intent, REQUEST_PICK)
    }

    private fun chooseFolder() {
        if (sources.isEmpty()) {
            Toast.makeText(this, "先选择笔刷文件", Toast.LENGTH_SHORT).show()
            return
        }
        if (formatList.isEmpty()) {
            Toast.makeText(this, "没有可用的输出格式", Toast.LENGTH_SHORT).show()
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

    private fun load(uris: List<Uri>) {
        for (uri in uris) {
            val name = displayName(uri)
            try {
                val bytes = contentResolver.openInputStream(uri).use { stream -> readFully(stream) }
                if (bytes == null || bytes.isEmpty()) {
                    log("✗ $name: 文件是空的")
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
                sources.add(Source(name, bytes, summary))
            } catch (error: Throwable) {
                log("✗ $name: ${error.message ?: error.toString()}")
            }
        }
        refreshSelection()
    }

    private fun readFully(stream: InputStream?): ByteArray? = stream?.use { it.readBytes() }

    private fun refreshSelection() {
        if (sources.isEmpty()) {
            selectedView.text = "还没有选择文件"
            convertButton.isEnabled = false
            return
        }
        selectedView.text = sources.joinToString("\n\n") { "• ${it.name}\n   ${it.summary}" }
        convertButton.isEnabled = true
    }

    private fun displayName(uri: Uri): String {
        contentResolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use { cursor ->
            if (cursor.moveToFirst()) {
                val index = cursor.getColumnIndex(OpenableColumns.DISPLAY_NAME)
                if (index >= 0) return cursor.getString(index)
            }
        }
        return uri.lastPathSegment ?: "brush"
    }

    private fun startConversion(tree: Uri) {
        val format = formatList.getOrNull(formatSpinner.selectedItemPosition) ?: return
        val inputs = ArrayList(sources)
        busy = true
        convertButton.isEnabled = false
        log("开始转换 ${inputs.size} 个文件 → ${format.label} (${format.name})")
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
                    log("✓ ${source.name} → ${names.size} 个文件")
                } catch (error: Throwable) {
                    log("✗ ${source.name}: ${error.message ?: error.toString()}")
                }
                dir.deleteRecursively()
            }
            cacheRoot.deleteRecursively()
            runOnUiThread {
                if (outputs.isEmpty()) {
                    log("没有生成任何文件")
                    busy = false
                    convertButton.isEnabled = sources.isNotEmpty()
                    return@runOnUiThread
                }
                Thread {
                    val written = copyInto(tree, outputs)
                    runOnUiThread {
                        log("完成：$written 个文件已写入所选文件夹")
                        Toast.makeText(this, "转换完成：$written 个文件", Toast.LENGTH_SHORT).show()
                        busy = false
                        convertButton.isEnabled = sources.isNotEmpty()
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
            log("! 读不到目标文件夹里的文件名：${error.message}")
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
                log("✗ 写入 $candidate 失败：${error.message}")
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

    companion object {
        private const val REQUEST_PICK = 1
        private const val REQUEST_FOLDER = 2
    }
}
