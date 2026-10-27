import { useI18n } from "../lib/i18n";

export function Profiles() {
  const { t } = useI18n();
  return (
    <div className="p-6">
      <h1 className="text-lg font-semibold">{t("profiles.title")}</h1>
      <p className="text-xs text-muted-foreground">{t("profiles.subtitle")}</p>
    </div>
  );
}
