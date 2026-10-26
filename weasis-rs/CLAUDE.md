# Catatan untuk sesi Claude — rewrite Rust

Folder ini berisi rewrite Weasis ke Rust (web). Kode Java di root repo (branch `master`,
Weasis 4.8.0-SNAPSHOT) adalah **acuan** yang hanya dibaca.

## Cara melanjutkan pekerjaan (wajib di awal setiap sesi)

1. Baca `weasis-rs/docs/PROGRESS.md` → lihat "Langkah berikutnya" dan "Sub-langkah".
2. Baca langkah itu di `weasis-rs/docs/PLAN.md` (scope, keputusan D1…, verifikasi), lalu
   file Java acuan yang disebut di langkah itu.
3. Cek `git status` dan `git log --oneline -5` untuk memastikan status sesuai PROGRESS.md.
   Jika ada perubahan belum di-commit, periksa dulu sebelum melanjutkan.
4. Kerjakan satu langkah → verifikasi → centang di PLAN.md → perbarui PROGRESS.md →
   commit `weasis-rs: [x.y] <ringkasan>` → push ke `claude/vibrant-bardeen-k1240f`.

## Aturan

- Jangan mengubah kode Java; hanya dibaca sebagai acuan.
- Keputusan baru dicatat di PLAN.md §2 (D12, D13, …); jangan diam-diam mengganti keputusan lama.
- Bahasa dokumen di `weasis-rs/docs/`: Indonesia. Bahasa kode & komentar: Inggris.
- Aturan dokumentasi di `CLAUDE.md` root (tanpa roadmap/status di `docs/`) berlaku untuk modul
  Java, bukan untuk `weasis-rs/docs/`, yang memang berisi rencana & progres atas permintaan pemilik repo.
