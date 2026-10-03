export type FileCategory =
  | "Videos"
  | "Audio"
  | "Documents"
  | "Archives"
  | "Programs"
  | "Other";

const CATEGORY_MAP: Record<string, FileCategory> = {
  // Videos
  mp4: "Videos",
  mkv: "Videos",
  avi: "Videos",
  mov: "Videos",
  wmv: "Videos",
  flv: "Videos",
  webm: "Videos",
  m4v: "Videos",
  ts: "Videos",
  "3gp": "Videos",

  // Audio
  mp3: "Audio",
  flac: "Audio",
  wav: "Audio",
  aac: "Audio",
  ogg: "Audio",
  m4a: "Audio",
  wma: "Audio",
  opus: "Audio",

  // Documents
  pdf: "Documents",
  docx: "Documents",
  doc: "Documents",
  xlsx: "Documents",
  xls: "Documents",
  pptx: "Documents",
  ppt: "Documents",
  txt: "Documents",
  epub: "Documents",
  csv: "Documents",
  md: "Documents",
  rtf: "Documents",

  // Archives
  zip: "Archives",
  rar: "Archives",
  "7z": "Archives",
  tar: "Archives",
  gz: "Archives",
  bz2: "Archives",
  xz: "Archives",
  iso: "Archives",
  dmg: "Archives",

  // Programs
  exe: "Programs",
  msi: "Programs",
  apk: "Programs",
  deb: "Programs",
  rpm: "Programs",
  appimage: "Programs",
};

export function getFileCategory(filenameOrUri: string): FileCategory {
  if (!filenameOrUri) return "Other";
  const clean = filenameOrUri.split("?")[0].split("#")[0];
  const parts = clean.split(".");
  if (parts.length <= 1) return "Other";
  const ext = parts.pop()?.toLowerCase() || "";
  return CATEGORY_MAP[ext] || "Other";
}
