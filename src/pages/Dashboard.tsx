import { useI18n } from "../lib/i18n";
import { useStore } from "../lib/store";
import { StatusBadge } from "../components/StatusBadge";

export function Dashboard() {
  const { t } = useI18n();
  const { status } = useStore();
  return (
    <div className="p-6">
      <h1 className="text-lg font-semibold">{t("dashboard.title")}</h1>
      <p className="text-xs text-muted-foreground">{t("dashboard.subtitle")}</p>
      <div className="mt-4">
        <StatusBadge status={status} />
      </div>
      <div className="mt-4 text-xs text-muted-foreground">{t("dashboard.startHint")}</div>
    </div>
  );
}
