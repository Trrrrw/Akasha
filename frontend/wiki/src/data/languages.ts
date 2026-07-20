export type LanguageCode = "zh-CN" | "en-US" | "ja-JP" | "ko-KR";

export type LanguageOption = {
  code: LanguageCode;
  label: string;
};

export const defaultLanguage: LanguageCode = "zh-CN";

export const languages: LanguageOption[] = [
  { code: "zh-CN", label: "简体中文" },
  { code: "en-US", label: "English" },
  { code: "ja-JP", label: "日本語" },
  { code: "ko-KR", label: "한국어" },
];

export function isLanguageCode(value: string): value is LanguageCode {
  return languages.some((language) => language.code === value);
}
