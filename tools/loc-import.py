#!/usr/bin/env python3
"""Import Rufus' own translations (res/loc/rufus.loc, GPLv3) into FlashCraft TSV
catalogs. Legal because FlashCraft is licensed GPLv3.

Usage: tools/loc-import.py <path-to-rufus.loc> [i18n-dir]

- Maps FlashCraft's English UI keys to Rufus message IDs, then extracts the
  per-locale translation for each ID.
- Keys without a Rufus counterpart keep any existing hand-written row in the
  target TSV (see HAND section below for languages added by hand).
- 'Show/Hide advanced ...' strings are composed from MSG_121/MSG_122 + the
  MSG_119/MSG_120 noun phrase, mirroring how Rufus builds them.
"""
import re, sys, os

# ---- our canonical key order (English source strings) -----------------------
KEYS = [
    "Settings", "Language",
    "Drive Properties", "Device", "Refresh", "Boot selection",
    "Disk or ISO image (Please select)", "SELECT", "Persistent partition size",
    "Format Options", "Image option", "Standard installation",
    "Partition scheme", "Target system", "BIOS or UEFI", "UEFI (non CSM)",
    "Volume label", "File system", "Cluster size",
    "Show advanced drive properties", "Hide advanced drive properties",
    "List USB Hard Drives",
    "Show advanced format options", "Hide advanced format options",
    "Add fixes for old BIOSes (extra partition, align, etc.)",
    "Enable runtime UEFI media validation",
    "Quick format", "Create extended label and icon files",
    "Check device for bad blocks",
    "Status:", "READY", "Log", "START", "OK", "Cancel", "Close",
    "WARNING: ALL DATA ON DEVICE '{0}' WILL BE DESTROYED",
    "To continue, click OK. To abort the operation, click Cancel.",
    "aborted by user",
    "Please select an image (SELECT)", "Please select a device",
    "write started", "verify started",
    "WRITE DONE", "VERIFY OK",
    "Selected '{0}'", "Device list refreshed", "no drive selected",
    "image inspection is not available in the browser",
    "device access is not available in the browser",
]

# key -> (msg_id, transform). transform codes:
#   up    -> uppercase the translation (buttons)
#   colon -> append ':'
#   ph    -> replace '%s' with '{0}'
MAP = {
    "Drive Properties": ("IDS_DRIVE_PROPERTIES_TXT", ""),
    "Device": ("IDS_DEVICE_TXT", ""),
    "Boot selection": ("IDS_BOOT_SELECTION_TXT", ""),
    "SELECT": ("IDC_SELECT", "up"),
    "Image option": ("IDS_IMAGE_OPTION_TXT", ""),
    "Partition scheme": ("IDS_PARTITION_TYPE_TXT", ""),
    "Target system": ("IDS_TARGET_SYSTEM_TXT", ""),
    "Volume label": ("IDS_LABEL_TXT", ""),
    "File system": ("IDS_FILE_SYSTEM_TXT", ""),
    "Cluster size": ("IDS_CLUSTER_SIZE_TXT", ""),
    "List USB Hard Drives": ("IDC_LIST_USB_HDD", ""),
    "Quick format": ("IDC_QUICK_FORMAT", ""),
    "Create extended label and icon files": ("IDC_EXTENDED_LABEL", ""),
    "Check device for bad blocks": ("IDC_BAD_BLOCKS", ""),
    "Add fixes for old BIOSes (extra partition, align, etc.)": ("IDC_OLD_BIOS_FIXES", ""),
    "Enable runtime UEFI media validation": ("IDC_UEFI_MEDIA_VALIDATION", ""),
    "Close": ("IDCANCEL", ""),
    "Status:": ("IDS_STATUS_TXT", "colon"),
    "READY": ("MSG_210", ""),
    "Log": ("IDD_LOG", ""),
    "START": ("IDC_START", "up"),
    "OK": ("IDOK", ""),
    "Cancel": ("MSG_007", ""),
    "BIOS or UEFI": ("MSG_033", ""),
    "UEFI (non CSM)": ("MSG_032", ""),
    "Persistent partition size": ("MSG_123", ""),
    "WARNING: ALL DATA ON DEVICE '{0}' WILL BE DESTROYED": ("MSG_003#0", "ph"),
    "To continue, click OK. To abort the operation, click Cancel.": ("MSG_003#1", ""),
    "Disk or ISO image (Please select)": ("MSG_281", "s=MSG_280"),
    "Show advanced drive properties": ("MSG_121", "s=MSG_119"),
    "Hide advanced drive properties": ("MSG_122", "s=MSG_119"),
    "Show advanced format options": ("MSG_121", "s=MSG_120"),
    "Hide advanced format options": ("MSG_122", "s=MSG_120"),
    "Format Options": ("IDS_FORMAT_OPTIONS_TXT", ""),
    # composed from MSG_121 ("Show %s") etc. handled by s= transform
}

# Hand-written translations for keys with no Rufus counterpart, for the
# languages whose catalogs did not exist before this import.
HAND = {
    "ar": {
        "Settings": "الإعدادات", "Language": "اللغة", "Refresh": "تحديث",
        "Standard installation": "تثبيت قياسي",
        "aborted by user": "تم الإلغاء من قبل المستخدم",
        "Please select an image (SELECT)": "الرجاء تحديد صورة (تحديد)",
        "Please select a device": "الرجاء تحديد جهاز",
        "write started": "بدأت الكتابة: {0}", "verify started": "بدأ التحقق: {0}",
        "WRITE DONE": "اكتملت الكتابة", "VERIFY OK": "تم التحقق بنجاح",
        "Selected '{0}'": "تم التحديد: '{0}'",
        "Device list refreshed": "تم تحديث قائمة الأجهزة",
        "no drive selected": "لم يتم تحديد جهاز",
        "image inspection is not available in the browser": "فحص الصورة غير متوفر في المتصفح",
        "device access is not available in the browser": "الوصول إلى الأجهزة غير متوفر في المتصفح",
    },
    "fa": {
        "Settings": "تنظیمات", "Language": "زبان", "Refresh": "به‌روزرسانی",
        "Standard installation": "نصب استاندارد",
        "aborted by user": "توسط کاربر لغو شد",
        "Please select an image (SELECT)": "لطفاً یک ایمیج انتخاب کنید (انتخاب)",
        "Please select a device": "لطفاً یک دستگاه انتخاب کنید",
        "write started": "نوشتن آغاز شد: {0}", "verify started": "تأیید آغاز شد: {0}",
        "WRITE DONE": "نوشتن تمام شد", "VERIFY OK": "تأیید موفق",
        "Selected '{0}'": "انتخاب شد: '{0}'",
        "Device list refreshed": "فهرست دستگاه‌ها به‌روزرسانی شد",
        "no drive selected": "دستگاهی انتخاب نشده",
        "image inspection is not available in the browser": "بررسی ایمیج در مرورگر در دسترس نیست",
        "device access is not available in the browser": "دسترسی به دستگاه در مرورگر در دسترس نیست",
    },
    "he": {
        "Settings": "הגדרות", "Language": "שפה", "Refresh": "רענן",
        "Standard installation": "התקנה רגילה",
        "aborted by user": "בוטל על ידי המשתמש",
        "Please select an image (SELECT)": "אנא בחר תמונה (בחר)",
        "Please select a device": "אנא בחר התקן",
        "write started": "הכתיבה החלה: {0}", "verify started": "האימות החל: {0}",
        "WRITE DONE": "הכתיבה הסתיימה", "VERIFY OK": "האימות הצליח",
        "Selected '{0}'": "נבחר: '{0}'",
        "Device list refreshed": "רשימת ההתקנים עודכנה",
        "no drive selected": "לא נבחר התקן",
        "image inspection is not available in the browser": "בדיקת תמונה אינה זמינה בדפדפן",
        "device access is not available in the browser": "גישה להתקנים אינה זמינה בדפדפן",
    },
    "id": {
        "Settings": "Pengaturan", "Language": "Bahasa", "Refresh": "Segarkan",
        "Standard installation": "Instalasi standar",
        "aborted by user": "dibatalkan oleh pengguna",
        "Please select an image (SELECT)": "Pilih image (PILIH)",
        "Please select a device": "Pilih perangkat",
        "write started": "penulisan dimulai: {0}", "verify started": "verifikasi dimulai: {0}",
        "WRITE DONE": "PENULISAN SELESAI", "VERIFY OK": "VERIFIKASI OK",
        "Selected '{0}'": "Terpilih: '{0}'",
        "Device list refreshed": "Daftar perangkat disegarkan",
        "no drive selected": "tidak ada perangkat terpilih",
        "image inspection is not available in the browser": "inspeksi image tidak tersedia di browser",
        "device access is not available in the browser": "akses perangkat tidak tersedia di browser",
    },
    "ja": {
        "Settings": "設定", "Language": "言語", "Refresh": "更新",
        "Standard installation": "標準インストール",
        "aborted by user": "ユーザーにより中止されました",
        "Please select an image (SELECT)": "イメージを選択してください（選択）",
        "Please select a device": "デバイスを選択してください",
        "write started": "書き込みを開始しました: {0}",
        "verify started": "検証を開始しました: {0}",
        "WRITE DONE": "書き込み完了", "VERIFY OK": "検証OK",
        "Selected '{0}'": "選択: '{0}'",
        "Device list refreshed": "デバイス一覧を更新しました",
        "no drive selected": "デバイスが選択されていません",
        "image inspection is not available in the browser": "イメージ検査はブラウザーでは利用できません",
        "device access is not available in the browser": "デバイスアクセスはブラウザーでは利用できません",
    },
    "ko": {
        "Settings": "설정", "Language": "언어", "Refresh": "새로 고침",
        "Standard installation": "표준 설치",
        "aborted by user": "사용자가 취소했습니다",
        "Please select an image (SELECT)": "이미지를 선택하세요 (선택)",
        "Please select a device": "디바이스를 선택하세요",
        "write started": "쓰기 시작됨: {0}", "verify started": "검증 시작됨: {0}",
        "WRITE DONE": "쓰기 완료", "VERIFY OK": "검증 완료",
        "Selected '{0}'": "선택됨: '{0}'",
        "Device list refreshed": "디바이스 목록이 새로 고쳐졌습니다",
        "no drive selected": "선택된 디바이스 없음",
        "image inspection is not available in the browser": "이미지 검사는 브라우저에서 사용할 수 없습니다",
        "device access is not available in the browser": "디바이스 액세스는 브라우저에서 사용할 수 없습니다",
    },
    "lt": {
        "Settings": "Nustatymai", "Language": "Kalba", "Refresh": "Atnaujinti",
        "Standard installation": "Standartinis diegimas",
        "aborted by user": "atšaukė vartotojas",
        "Please select an image (SELECT)": "Pasirinkite atvaizdą (PASIRINKTI)",
        "Please select a device": "Pasirinkite įrenginį",
        "write started": "pradėtas rašymas: {0}", "verify started": "pradėtas tikrinimas: {0}",
        "WRITE DONE": "RAŠYMAS BAIGTAS", "VERIFY OK": "TIKRINIMAS SĖKMINGAS",
        "Selected '{0}'": "Pasirinkta: '{0}'",
        "Device list refreshed": "Įrenginių sąrašas atnaujintas",
        "no drive selected": "nepasirinktas įrenginys",
        "image inspection is not available in the browser": "atvaizdo tikrinimas nepasiekiamas naršyklėje",
        "device access is not available in the browser": "prieiga prie įrenginių nepasiekiama naršyklėje",
    },
    "lv": {
        "Settings": "Iestatījumi", "Language": "Valoda", "Refresh": "Atsvaidzināt",
        "Standard installation": "Standarta instalācija",
        "aborted by user": "lietotājs atcēla",
        "Please select an image (SELECT)": "Lūdzu, atlasiet attēlu (ATLASĪT)",
        "Please select a device": "Lūdzu, atlasiet ierīci",
        "write started": "rakstīšana sākta: {0}", "verify started": "verifikācija sākta: {0}",
        "WRITE DONE": "RAKSTĪŠANA PABEIGTA", "VERIFY OK": "VERIFIKĀCIJA VEIKSMĪGA",
        "Selected '{0}'": "Atlasīts: '{0}'",
        "Device list refreshed": "Ierīču saraksts atsvaidzināts",
        "no drive selected": "nav atlasīta ierīce",
        "image inspection is not available in the browser": "attēla pārbaude nav pieejama pārlūkā",
        "device access is not available in the browser": "piekļuve ierīcēm nav pieejama pārlūkā",
    },
    "ms": {
        "Settings": "Tetapan", "Language": "Bahasa", "Refresh": "Segar semula",
        "Standard installation": "Pemasangan standard",
        "Status:": "Keadaan:", "Log": "Log", "OK": "OK",
        "aborted by user": "dibatalkan oleh pengguna",
        "Please select an image (SELECT)": "Sila pilih imej (PILIH)",
        "Please select a device": "Sila pilih peranti",
        "write started": "penulisan bermula: {0}", "verify started": "pengesahan bermula: {0}",
        "WRITE DONE": "PENULISAN SELESAI", "VERIFY OK": "PENGESAHAN OK",
        "Selected '{0}'": "Terpilih: '{0}'",
        "Device list refreshed": "Senarai peranti disegarkan",
        "no drive selected": "tiada peranti dipilih",
        "image inspection is not available in the browser": "pemeriksaan imej tidak tersedia dalam pelayar",
        "device access is not available in the browser": "akses peranti tidak tersedia dalam pelayar",
    },
    "th": {
        "Settings": "การตั้งค่า", "Language": "ภาษา", "Refresh": "รีเฟรช",
        "Standard installation": "การติดตั้งแบบมาตรฐาน",
        "aborted by user": "ผู้ใช้ยกเลิก",
        "Please select an image (SELECT)": "โปรดเลือกอิมเมจ (เลือก)",
        "Please select a device": "โปรดเลือกอุปกรณ์",
        "write started": "เริ่มเขียน: {0}", "verify started": "เริ่มตรวจสอบ: {0}",
        "WRITE DONE": "เขียนเสร็จสิ้น", "VERIFY OK": "ตรวจสอบสำเร็จ",
        "Selected '{0}'": "เลือกแล้ว: '{0}'",
        "Device list refreshed": "รายการอุปกรณ์ได้รับการรีเฟรช",
        "no drive selected": "ไม่ได้เลือกอุปกรณ์",
        "image inspection is not available in the browser": "การตรวจสอบอิมเมจไม่พร้อมใช้งานในเบราว์เซอร์",
        "device access is not available in the browser": "ไม่สามารถเข้าถึงอุปกรณ์ในเบราว์เซอร์",
    },
    "vi": {
        "Settings": "Cài đặt", "Language": "Ngôn ngữ", "Refresh": "Làm mới",
        "Standard installation": "Cài đặt tiêu chuẩn",
        "aborted by user": "người dùng đã hủy",
        "Please select an image (SELECT)": "Vui lòng chọn ảnh (CHỌN)",
        "Please select a device": "Vui lòng chọn thiết bị",
        "write started": "bắt đầu ghi: {0}", "verify started": "bắt đầu xác minh: {0}",
        "WRITE DONE": "GHI HOÀN TẤT", "VERIFY OK": "XÁC MINH OK",
        "Selected '{0}'": "Đã chọn: '{0}'",
        "Device list refreshed": "Đã làm mới danh sách thiết bị",
        "no drive selected": "chưa chọn thiết bị",
        "image inspection is not available in the browser": "kiểm tra ảnh không khả dụng trong trình duyệt",
        "device access is not available in the browser": "truy cập thiết bị không khả dụng trong trình duyệt",
    },
    "zh-CN": {
        "Settings": "设置", "Language": "语言", "Refresh": "刷新",
        "Standard installation": "标准安装",
        "aborted by user": "已被用户中止",
        "Please select an image (SELECT)": "请选择镜像（选择）",
        "Please select a device": "请选择设备",
        "write started": "已开始写入：{0}", "verify started": "已开始校验：{0}",
        "WRITE DONE": "写入完成", "VERIFY OK": "校验通过",
        "Selected '{0}'": "已选择：'{0}'",
        "Device list refreshed": "设备列表已刷新",
        "no drive selected": "未选择设备",
        "image inspection is not available in the browser": "镜像检查在浏览器中不可用",
        "device access is not available in the browser": "浏览器中无法访问设备",
    },
    "zh-TW": {
        "Settings": "設定", "Language": "語言", "Refresh": "重新整理",
        "Standard installation": "標準安裝",
        "aborted by user": "已被使用者中止",
        "Please select an image (SELECT)": "請選擇映像（選擇）",
        "Please select a device": "請選擇裝置",
        "write started": "已開始寫入：{0}", "verify started": "已開始驗證：{0}",
        "WRITE DONE": "寫入完成", "VERIFY OK": "驗證通過",
        "Selected '{0}'": "已選擇：'{0}'",
        "Device list refreshed": "裝置清單已重新整理",
        "no drive selected": "未選擇裝置",
        "image inspection is not available in the browser": "映像檢查在瀏覽器中不可用",
        "device access is not available in the browser": "瀏覽器中無法存取裝置",
    },
}

def parse_loc(path):
    locales, cur = {}, None
    for raw in open(path, encoding="utf-8"):
        line = raw.rstrip("\n")
        m = re.match(r'l "([a-z]{2}(?:-[A-Z]{2})?)"', line)
        if m:
            cur = m.group(1)
            locales[cur] = {}
            continue
        m = re.match(r't ([A-Z0-9_]+) "(.*)"', line)
        if m and cur:
            locales[cur][m.group(1)] = m.group(2)
            continue
        m = re.match(r'\s+"(.*)"', line)
        if m and cur and locales[cur]:
            last = next(reversed(locales[cur]))
            locales[cur][last] += m.group(1)
    return locales

def get(loc_map, ref):
    mid, _, frag = ref.partition("#")
    val = loc_map.get(mid)
    if val is None:
        return None
    if frag:
        parts = val.split("\\n")
        return parts[int(frag)] if int(frag) < len(parts) else None
    return val

def apply_transform(text, spec, loc_map):
    if text is None:
        return None
    if spec.startswith("s="):
        sub = get(loc_map, spec[2:])
        text = text.replace("%s", sub if sub is not None else "")
        spec = ""
    if "ph" in spec:
        text = text.replace("%s", "{0}")
    if "colon" in spec:
        text = text.rstrip(":") + ":"
    if "up" in spec:
        text = text.upper()
    return text.replace('\\"', '"').strip()

def read_tsv(path):
    rows = {}
    if os.path.exists(path):
        for line in open(path, encoding="utf-8"):
            if "\t" in line:
                k, v = line.rstrip("\n").split("\t", 1)
                rows[k] = v
    return rows

def main():
    loc_path = sys.argv[1]
    out_dir = sys.argv[2] if len(sys.argv) > 2 else "crates/ui-egui/src/i18n"
    locales = parse_loc(loc_path)
    en = locales["en-US"]
    name_to_file = {
        "ar-SA": "ar", "bg-BG": "bg", "cs-CZ": "cs", "da-DK": "da",
        "de-DE": "de", "el-GR": "el", "es-ES": "es", "fa-IR": "fa",
        "fi-FI": "fi", "fr-FR": "fr", "he-IL": "he", "hr-HR": "hr",
        "hu-HU": "hu", "id-ID": "id", "it-IT": "it", "ja-JP": "ja",
        "ko-KR": "ko", "lt-LT": "lt", "lv-LV": "lv", "ms-MY": "ms",
        "nb-NO": "nb", "nl-NL": "nl", "pl-PL": "pl", "pt-BR": "pt-BR",
        "pt-PT": "pt-PT", "ro-RO": "ro", "ru-RU": "ru", "sk-SK": "sk",
        "sl-SI": "sl", "sr-RS": "sr", "sv-SE": "sv", "th-TH": "th",
        "tr-TR": "tr", "uk-UA": "uk", "vi-VN": "vi", "zh-CN": "zh-CN",
        "zh-TW": "zh-TW",
    }
    os.makedirs(out_dir, exist_ok=True)
    for code, fname in sorted(name_to_file.items(), key=lambda x: x[1]):
        loc_map = locales.get(code, {})
        existing = read_tsv(os.path.join(out_dir, fname + ".tsv"))
        rows, from_loc, from_hand = {}, 0, 0
        for key in KEYS:
            # Rows already in the catalog win (they may carry hand edits or
            # reviewer corrections); rufus.loc fills only missing keys.
            val = existing.get(key)
            if not val and key in MAP:
                mid, spec = MAP[key]
                val = apply_transform(get(loc_map, mid), spec, loc_map)
                if val:
                    from_loc += 1
            if not val:
                val = HAND.get(fname, {}).get(key)
            if val and key not in MAP:
                from_hand += 1
            if val:
                rows[key] = val
        path = os.path.join(out_dir, fname + ".tsv")
        with open(path, "w", encoding="utf-8") as f:
            for k in KEYS:
                if k in rows:
                    f.write(f"{k}\t{rows[k]}\n")
        print(f"{fname:6s} loc={from_loc:2d} hand={from_hand:2d} total={len(rows):2d}")

if __name__ == "__main__":
    main()
