use chrono::{DateTime, NaiveDate, Utc};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct UnpaidBillItem {
    pub effective_date: Option<NaiveDate>,
    pub description: String,
    pub quantity: Option<String>,
    pub unit_price_idr: Option<i64>,
    pub debit_idr: i64,
    pub allocated_idr: i64,
    pub remaining_idr: i64,
}

#[derive(Debug, Clone)]
pub struct CustomerBillData {
    pub customer_id: Uuid,
    pub customer_code: Option<String>,
    pub customer_name: String,
    pub customer_address: Option<String>,
    pub printed_at: DateTime<Utc>,
    pub items: Vec<UnpaidBillItem>,
}

fn sanitize_pdf_text(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '(' => out.push_str("\\("),
            ')' => out.push_str("\\)"),
            c if c.is_ascii() && !c.is_ascii_control() => out.push(c),
            _ => out.push(' '),
        }
    }
    out
}

pub fn format_idr_dots(val: i64) -> String {
    let is_neg = val < 0;
    let s = val.abs().to_string();
    let mut out = String::new();
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    for (i, &c) in chars.iter().enumerate() {
        if i > 0 && (len - i).is_multiple_of(3) {
            out.push('.');
        }
        out.push(c);
    }
    if is_neg { format!("-{}", out) } else { out }
}

pub fn generate_customer_bill_pdf(bill: &CustomerBillData) -> Vec<u8> {
    // Dimensi halaman A4: 595.28 pt x 841.89 pt
    let page_width = 595.28_f32;
    let page_height = 841.89_f32;
    let margin_left = 28.0_f32;
    let margin_right = 567.0_f32; // lebar tabel = 539 pt

    // Kolom tabel: No (22), Tanggal (58), Uraian (165), Qty (32), Harga (62), Debet (66), Kredit (66), Sisa (68) = 539 pt
    let col_x = [
        margin_left,         // No: 28
        margin_left + 22.0,  // Tanggal: 50
        margin_left + 80.0,  // Uraian: 108
        margin_left + 245.0, // Qty: 273
        margin_left + 277.0, // Harga: 305
        margin_left + 339.0, // Debet: 367
        margin_left + 405.0, // Kredit: 433
        margin_left + 471.0, // Sisa: 499
        margin_right,        // Ujung kanan: 567
    ];

    let row_height = 18.0_f32;
    let bottom_limit = 70.0_f32; // Batas bawah sebelum ganti halaman

    // Siapkan halaman-halaman (tiap halaman berisi potongan konten PDF stream)
    let mut pages_content: Vec<String> = Vec::new();
    let mut current_page_stream = String::new();

    let mut current_y = 800.0_f32;

    // Helper untuk header halaman pertama
    let draw_first_page_header = |stream: &mut String, y: &mut f32| {
        // Kop Toko
        stream.push_str("0.05 0.1 0.2 rg\n"); // Slate gelap
        stream.push_str("BT /F2 15 Tf ");
        stream.push_str(&format!(
            "{:.2} {:.2} Td (KEUANGAN TOKO - REZA PUTRA) Tj ET\n",
            margin_left, *y
        ));
        *y -= 14.0;

        stream.push_str("0.35 0.4 0.45 rg\n");
        stream.push_str("BT /F1 9 Tf ");
        stream.push_str(&format!(
            "{:.2} {:.2} Td (Sistem Pencatatan Pembukuan & Piutang Pelanggan) Tj ET\n",
            margin_left, *y
        ));
        *y -= 14.0;

        // Garis batas kop
        stream.push_str("0.8 0.85 0.9 RG 1.5 w ");
        stream.push_str(&format!(
            "{:.2} {:.2} m {:.2} {:.2} l S\n",
            margin_left, *y, margin_right, *y
        ));
        *y -= 18.0;

        // Judul Dokumen
        stream.push_str("0.08 0.35 0.25 rg\n"); // Emerald gelap
        stream.push_str("BT /F2 13 Tf ");
        stream.push_str(&format!(
            "{:.2} {:.2} Td (RINCIAN TAGIHAN PIUTANG BELUM LUNAS) Tj ET\n",
            margin_left, *y
        ));
        *y -= 14.0;

        // Kotak Informasi Pelanggan & Tanggal Cetak
        let box_y = *y - 50.0;
        let box_height = 50.0;
        stream.push_str("0.96 0.98 1.0 rg 0.82 0.88 0.95 RG 0.75 w ");
        stream.push_str(&format!(
            "{:.2} {:.2} {:.2} {:.2} re B\n",
            margin_left,
            box_y,
            margin_right - margin_left,
            box_height
        ));

        // Teks Informasi Pelanggan (Kolom Kiri)
        let cust_code = bill.customer_code.as_deref().unwrap_or("-");
        let cust_addr = bill.customer_address.as_deref().unwrap_or("-");
        let print_date_str = bill.printed_at.format("%d/%m/%Y %H:%M WIB").to_string();

        stream.push_str("0.1 0.15 0.2 rg\n");
        stream.push_str("BT /F2 9.5 Tf ");
        stream.push_str(&format!(
            "{:.2} {:.2} Td (Pelanggan : {}) Tj ET\n",
            margin_left + 8.0,
            *y - 14.0,
            sanitize_pdf_text(&bill.customer_name)
        ));
        stream.push_str("BT /F1 8.5 Tf ");
        stream.push_str(&format!(
            "{:.2} {:.2} Td (Kode : {}   |   Alamat : {}) Tj ET\n",
            margin_left + 8.0,
            *y - 27.0,
            sanitize_pdf_text(cust_code),
            sanitize_pdf_text(cust_addr)
        ));

        // Teks Informasi Dokumen (Kolom Kanan)
        stream.push_str("BT /F1 8.5 Tf ");
        stream.push_str(&format!(
            "{:.2} {:.2} Td (Tanggal Cetak : {}) Tj ET\n",
            margin_left + 330.0,
            *y - 14.0,
            print_date_str
        ));
        stream.push_str("BT /F2 8.5 Tf 0.8 0.1 0.1 rg ");
        stream.push_str(&format!(
            "{:.2} {:.2} Td (Status : Tagihan Belum Lunas) Tj ET\n",
            margin_left + 330.0,
            *y - 27.0
        ));

        *y -= 62.0;
    };

    // Helper untuk header tabel di tiap halaman
    let draw_table_header = |stream: &mut String, y: &mut f32| {
        let th_height = 20.0_f32;
        *y -= th_height;

        // Background header tabel (Slate 800)
        stream.push_str("0.12 0.18 0.25 rg 0.12 0.18 0.25 RG 0.5 w ");
        stream.push_str(&format!(
            "{:.2} {:.2} {:.2} {:.2} re B\n",
            margin_left,
            *y,
            margin_right - margin_left,
            th_height
        ));

        // Teks header tabel
        stream.push_str("1.0 1.0 1.0 rg BT /F2 8.5 Tf\n");
        let th_y = *y + 6.0;

        // No
        stream.push_str(&format!("{:.2} {:.2} Td (No) Tj\n", col_x[0] + 5.0, th_y));
        // Tanggal
        stream.push_str(&format!(
            "1 0 0 1 {:.2} {:.2} Tm (Tanggal) Tj\n",
            col_x[1] + 4.0,
            th_y
        ));
        // Uraian
        stream.push_str(&format!(
            "1 0 0 1 {:.2} {:.2} Tm (Uraian Transaksi) Tj\n",
            col_x[2] + 4.0,
            th_y
        ));
        // Qty
        stream.push_str(&format!(
            "1 0 0 1 {:.2} {:.2} Tm (Qty) Tj\n",
            col_x[3] + 4.0,
            th_y
        ));
        // Harga Satuan
        stream.push_str(&format!(
            "1 0 0 1 {:.2} {:.2} Tm (Harga Satuan) Tj\n",
            col_x[4] + 4.0,
            th_y
        ));
        // Debet (Rp)
        stream.push_str(&format!(
            "1 0 0 1 {:.2} {:.2} Tm (Debet \\(Rp\\)) Tj\n",
            col_x[5] + 6.0,
            th_y
        ));
        // Kredit (Rp)
        stream.push_str(&format!(
            "1 0 0 1 {:.2} {:.2} Tm (Kredit \\(Rp\\)) Tj\n",
            col_x[6] + 6.0,
            th_y
        ));
        // Saldo Berjalan (Rp)
        stream.push_str(&format!(
            "1 0 0 1 {:.2} {:.2} Tm (Saldo Berjalan \\(Rp\\)) Tj\n",
            col_x[7] + 4.0,
            th_y
        ));

        stream.push_str("ET\n");
    };

    // Render halaman pertama
    draw_first_page_header(&mut current_page_stream, &mut current_y);
    draw_table_header(&mut current_page_stream, &mut current_y);

    let mut total_debit = 0_i64;
    let mut total_allocated = 0_i64;
    let mut total_remaining = 0_i64;

    if bill.items.is_empty() {
        // Tampilkan pesan kosong jika tidak ada piutang
        current_y -= row_height + 4.0;
        current_page_stream.push_str("0.98 0.98 0.98 rg 0.85 0.85 0.85 RG 0.5 w ");
        current_page_stream.push_str(&format!(
            "{:.2} {:.2} {:.2} {:.2} re B\n",
            margin_left,
            current_y,
            margin_right - margin_left,
            row_height + 4.0
        ));
        current_page_stream.push_str("0.2 0.5 0.3 rg BT /F2 9 Tf ");
        current_page_stream.push_str(&format!("{:.2} {:.2} Td (Tidak ada tagihan piutang belum lunas. Semua transaksi telah lunas.) Tj ET\n", margin_left + 14.0, current_y + 6.0));
    } else {
        for (idx, item) in bill.items.iter().enumerate() {
            total_debit += item.debit_idr;
            total_allocated += item.allocated_idr;
            total_remaining += item.remaining_idr;

            // Periksa apakah halaman penuh
            if current_y - row_height < bottom_limit {
                pages_content.push(current_page_stream);
                current_page_stream = String::new();
                current_y = 800.0;

                // Sub-header halaman lanjutan
                current_page_stream.push_str("0.3 0.35 0.4 rg BT /F2 9 Tf ");
                current_page_stream.push_str(&format!(
                    "{:.2} {:.2} Td (RINCIAN TAGIHAN PIUTANG - {} \\(Lanjutan\\)) Tj ET\n",
                    margin_left,
                    current_y,
                    sanitize_pdf_text(&bill.customer_name)
                ));
                current_y -= 8.0;

                draw_table_header(&mut current_page_stream, &mut current_y);
            }

            current_y -= row_height;

            // Background berselang-seling (Zebra striping)
            if idx % 2 == 1 {
                current_page_stream.push_str("0.96 0.97 0.98 rg 0.96 0.97 0.98 RG 0.5 w ");
                current_page_stream.push_str(&format!(
                    "{:.2} {:.2} {:.2} {:.2} re B\n",
                    margin_left,
                    current_y,
                    margin_right - margin_left,
                    row_height
                ));
            }

            // Garis pembatas baris
            current_page_stream.push_str("0.88 0.9 0.92 RG 0.4 w ");
            current_page_stream.push_str(&format!(
                "{:.2} {:.2} m {:.2} {:.2} l S\n",
                margin_left, current_y, margin_right, current_y
            ));

            // Isi Kolom Data
            let text_y = current_y + 5.0;
            current_page_stream.push_str("0.1 0.15 0.2 rg BT /F1 8 Tf\n");

            // 1. No
            let no_str = format!("{}", idx + 1);
            current_page_stream.push_str(&format!(
                "{:.2} {:.2} Td ({}) Tj\n",
                col_x[0] + 4.0,
                text_y,
                no_str
            ));

            // 2. Tanggal
            let date_str = item
                .effective_date
                .map(|d| d.format("%d/%m/%Y").to_string())
                .unwrap_or_else(|| "-".into());
            current_page_stream.push_str(&format!(
                "1 0 0 1 {:.2} {:.2} Tm ({}) Tj\n",
                col_x[1] + 3.0,
                text_y,
                date_str
            ));

            // 3. Uraian Transaksi (maksimal ~30 karakter agar tidak menabrak batas kolom)
            let mut desc_clean = sanitize_pdf_text(&item.description);
            if desc_clean.chars().count() > 33 {
                desc_clean = format!("{}...", desc_clean.chars().take(30).collect::<String>());
            }
            current_page_stream.push_str(&format!(
                "1 0 0 1 {:.2} {:.2} Tm ({}) Tj\n",
                col_x[2] + 4.0,
                text_y,
                desc_clean
            ));

            // 4. Qty
            let qty_clean = item.quantity.as_deref().unwrap_or("1");
            current_page_stream.push_str(&format!(
                "1 0 0 1 {:.2} {:.2} Tm ({}) Tj\n",
                col_x[3] + 4.0,
                text_y,
                sanitize_pdf_text(qty_clean)
            ));

            // 5. Harga Satuan
            let price_str = item
                .unit_price_idr
                .map(format_idr_dots)
                .unwrap_or_else(|| "-".into());
            current_page_stream.push_str(&format!(
                "1 0 0 1 {:.2} {:.2} Tm ({}) Tj\n",
                col_x[4] + 8.0,
                text_y,
                price_str
            ));

            // 6. Debet
            let debit_str = format_idr_dots(item.debit_idr);
            current_page_stream.push_str(&format!(
                "1 0 0 1 {:.2} {:.2} Tm ({}) Tj\n",
                col_x[5] + 8.0,
                text_y,
                debit_str
            ));

            // 7. Kredit (Terbayar)
            let alloc_str = if item.allocated_idr > 0 {
                format_idr_dots(item.allocated_idr)
            } else {
                "-".into()
            };
            current_page_stream.push_str(&format!(
                "1 0 0 1 {:.2} {:.2} Tm ({}) Tj\n",
                col_x[6] + 8.0,
                text_y,
                alloc_str
            ));

            // 8. Sisa Tagihan (Warna Merah Bold)
            let rem_str = format_idr_dots(item.remaining_idr);
            current_page_stream.push_str("ET\n"); // Akhiri font regular
            current_page_stream.push_str("0.75 0.1 0.1 rg BT /F2 8 Tf\n");
            current_page_stream.push_str(&format!(
                "{:.2} {:.2} Td ({}) Tj ET\n",
                col_x[7] + 8.0,
                text_y,
                rem_str
            ));
        }

        // Baris Total Tabel
        current_y -= row_height + 2.0;
        current_page_stream.push_str("0.9 0.93 0.96 rg 0.7 0.75 0.8 RG 0.8 w ");
        current_page_stream.push_str(&format!(
            "{:.2} {:.2} {:.2} {:.2} re B\n",
            margin_left,
            current_y,
            margin_right - margin_left,
            row_height + 2.0
        ));

        let tot_y = current_y + 6.0;
        current_page_stream.push_str("0.05 0.1 0.15 rg BT /F2 8.5 Tf\n");
        current_page_stream.push_str(&format!(
            "{:.2} {:.2} Td (TOTAL SISA TAGIHAN) Tj\n",
            col_x[0] + 6.0,
            tot_y
        ));
        current_page_stream.push_str(&format!(
            "1 0 0 1 {:.2} {:.2} Tm ({}) Tj\n",
            col_x[5] + 8.0,
            tot_y,
            format_idr_dots(total_debit)
        ));
        current_page_stream.push_str(&format!(
            "1 0 0 1 {:.2} {:.2} Tm ({}) Tj\n",
            col_x[6] + 8.0,
            tot_y,
            format_idr_dots(total_allocated)
        ));
        current_page_stream.push_str("ET\n");

        current_page_stream.push_str("0.8 0.05 0.05 rg BT /F2 8.5 Tf\n");
        current_page_stream.push_str(&format!(
            "{:.2} {:.2} Td (Rp {}) Tj ET\n",
            col_x[7] + 8.0,
            tot_y,
            format_idr_dots(total_remaining)
        ));
    }

    // Kotak Ringkasan Besar dan Tanda Tangan
    if current_y - 120.0 < bottom_limit {
        pages_content.push(current_page_stream);
        current_page_stream = String::new();
        current_y = 800.0;
    }

    current_y -= 45.0;
    let card_y = current_y;
    // Kotak total nominal terbilang / ringkas
    current_page_stream.push_str("0.97 0.98 1.0 rg 0.75 0.82 0.92 RG 0.8 w ");
    current_page_stream.push_str(&format!(
        "{:.2} {:.2} 320.0 38.0 re B\n",
        margin_left, card_y
    ));
    current_page_stream.push_str("0.1 0.2 0.3 rg BT /F1 8.5 Tf ");
    current_page_stream.push_str(&format!(
        "{:.2} {:.2} Td (Total yang harus dibayar oleh pelanggan:) Tj ET\n",
        margin_left + 10.0,
        card_y + 22.0
    ));
    current_page_stream.push_str("0.8 0.1 0.1 rg BT /F2 12 Tf ");
    current_page_stream.push_str(&format!(
        "{:.2} {:.2} Td (Rp {}) Tj ET\n",
        margin_left + 10.0,
        card_y + 8.0,
        format_idr_dots(total_remaining)
    ));

    // Area Tanda Tangan
    let sig_y = card_y - 20.0;
    current_page_stream.push_str("0.1 0.15 0.2 rg BT /F1 8.5 Tf\n");
    // Tanda tangan pelanggan
    current_page_stream.push_str(&format!(
        "{:.2} {:.2} Td (Penerima / Pelanggan,) Tj\n",
        margin_left + 40.0,
        sig_y
    ));
    current_page_stream.push_str(&format!(
        "1 0 0 1 {:.2} {:.2} Tm (\\( ......................................... \\)) Tj\n",
        margin_left + 20.0,
        sig_y - 45.0
    ));

    // Tanda tangan pengelola toko
    current_page_stream.push_str(&format!(
        "1 0 0 1 {:.2} {:.2} Tm (Hormat Kami, Kasir / Toko) Tj\n",
        margin_left + 360.0,
        sig_y
    ));
    current_page_stream.push_str(&format!(
        "1 0 0 1 {:.2} {:.2} Tm (\\( ......................................... \\)) Tj\n",
        margin_left + 340.0,
        sig_y - 45.0
    ));
    current_page_stream.push_str("ET\n");

    // Catatan kaki
    let note_y = sig_y - 65.0;
    current_page_stream.push_str("0.45 0.5 0.55 rg BT /F1 7.5 Tf ");
    current_page_stream.push_str(&format!("{:.2} {:.2} Td (* Surat rincian ini adalah bukti resmi kewajiban piutang yang tercatat pada sistem pembukuan toko.) Tj ET\n", margin_left, note_y));

    // Simpan halaman terakhir
    pages_content.push(current_page_stream);

    // Sekarang rakit dokumen PDF 1.4 lengkap
    assemble_pdf_document(page_width, page_height, pages_content)
}

fn assemble_pdf_document(page_width: f32, page_height: f32, pages_stream: Vec<String>) -> Vec<u8> {
    let mut out = Vec::new();
    let total_pages = pages_stream.len();

    // 1. Header PDF
    out.extend_from_slice(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");

    let mut object_offsets: Vec<usize> = Vec::new();
    let mut current_obj_id = 1;

    // Objek 1: Catalog
    object_offsets.push(out.len());
    let catalog = format!(
        "{} 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n",
        current_obj_id
    );
    out.extend_from_slice(catalog.as_bytes());
    current_obj_id += 1;

    // Objek 2: Pages root
    // Page IDs: obj 3, obj 4, dst.
    let page_obj_ids: Vec<usize> = (0..total_pages).map(|i| 3 + i * 2).collect();
    let kids_str = page_obj_ids
        .iter()
        .map(|id| format!("{} 0 R", id))
        .collect::<Vec<_>>()
        .join(" ");
    object_offsets.push(out.len());
    let pages_root = format!(
        "{} 0 obj\n<< /Type /Pages /Kids [{}] /Count {} >>\nendobj\n",
        current_obj_id, kids_str, total_pages
    );
    out.extend_from_slice(pages_root.as_bytes());

    // Objek Font F1 (Helvetica) dan F2 (Helvetica-Bold) diletakkan di akhir
    let font_f1_id = 3 + total_pages * 2;
    let font_f2_id = font_f1_id + 1;

    // Tulis objek Page dan Contents stream untuk tiap halaman
    for (i, mut stream_content) in pages_stream.into_iter().enumerate() {
        // Tambahkan nomor halaman di footer tiap halaman
        let footer_text = format!("Halaman {} dari {}", i + 1, total_pages);
        stream_content.push_str(&format!(
            "0.5 0.55 0.6 rg BT /F1 7.5 Tf 1 0 0 1 480.00 25.00 Tm ({}) Tj ET\n",
            footer_text
        ));

        let page_id = 3 + i * 2;
        let content_id = page_id + 1;

        // Objek Page
        object_offsets.push(out.len());
        let page_def = format!(
            "{} 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {:.2} {:.2}] /Resources << /Font << /F1 {} 0 R /F2 {} 0 R >> >> /Contents {} 0 R >>\nendobj\n",
            page_id, page_width, page_height, font_f1_id, font_f2_id, content_id
        );
        out.extend_from_slice(page_def.as_bytes());

        // Objek Content Stream
        object_offsets.push(out.len());
        let stream_bytes = stream_content.as_bytes();
        let content_def = format!(
            "{} 0 obj\n<< /Length {} >>\nstream\n{}\nendstream\nendobj\n",
            content_id,
            stream_bytes.len(),
            stream_content
        );
        out.extend_from_slice(content_def.as_bytes());
    }

    // Objek Font F1
    object_offsets.push(out.len());
    let font_f1 = format!(
        "{} 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n",
        font_f1_id
    );
    out.extend_from_slice(font_f1.as_bytes());

    // Objek Font F2
    object_offsets.push(out.len());
    let font_f2 = format!(
        "{} 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold >>\nendobj\n",
        font_f2_id
    );
    out.extend_from_slice(font_f2.as_bytes());

    // XREF Table
    let xref_offset = out.len();
    let total_objects = font_f2_id;
    out.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", total_objects + 1).as_bytes(),
    );

    for offset in object_offsets {
        out.extend_from_slice(format!("{:010} 00000 n \n", offset).as_bytes());
    }

    // Trailer
    let trailer = format!(
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
        total_objects + 1,
        xref_offset
    );
    out.extend_from_slice(trailer.as_bytes());

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_valid_pdf_structure_and_header() {
        let bill = CustomerBillData {
            customer_id: Uuid::new_v4(),
            customer_code: Some("P017".into()),
            customer_name: "DESA SUMBARANG".into(),
            customer_address: Some("Kecamatan Jatinegara".into()),
            printed_at: Utc::now(),
            items: vec![
                UnpaidBillItem {
                    effective_date: NaiveDate::from_ymd_opt(2026, 11, 25),
                    description: "Kertas F4 1 Rim".into(),
                    quantity: Some("1".into()),
                    unit_price_idr: Some(55000),
                    debit_idr: 55000,
                    allocated_idr: 0,
                    remaining_idr: 55000,
                },
                UnpaidBillItem {
                    effective_date: NaiveDate::from_ymd_opt(2026, 11, 26),
                    description: "Spidol Whiteboard".into(),
                    quantity: Some("4".into()),
                    unit_price_idr: Some(10000),
                    debit_idr: 40000,
                    allocated_idr: 10000,
                    remaining_idr: 30000,
                },
            ],
        };

        let pdf = generate_customer_bill_pdf(&bill);
        assert!(pdf.starts_with(b"%PDF-1.4"));
        assert!(pdf.ends_with(b"%%EOF\n"));
        let pdf_str = String::from_utf8_lossy(&pdf);
        assert!(pdf_str.contains("/Catalog"));
        assert!(pdf_str.contains("/Pages"));
        assert!(pdf_str.contains("DESA SUMBARANG"));
        assert!(pdf_str.contains("55.000"));
        assert!(pdf_str.contains("85.000")); // Total sisa tagihan 55.000 + 30.000
    }
}
