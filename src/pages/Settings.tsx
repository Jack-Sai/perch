import { useI18n } from "../lib/i18n";

export function Settings() {
  const { t } = useI18n();
  return (
    <div className="p-6">
      <h1 className="text-lg font-semibold">{t("settings.title")}</h1>
      <p className="text-xs text-muted-foreground">{t("settings.subtitle")}</p>
    </div>
  );
}
