import { useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import {
  Download,
  FileDown,
  FolderCog,
  Plus,
  Radio,
  Trash2,
  Upload,
  CheckCircle2,
} from "lucide-react";
import { api } from "../lib/api";
import { useI18n } from "../lib/i18n";
import { useStore } from "../lib/store";
import { useToast } from "../lib/toast";
import { basename, cn } from "../lib/utils";
import { Badge } from "../components/ui/badge";
import { Button } from "../components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "../components/ui/card";
import { Dialog } from "../components/ui/dialog";
import { Input } from "../components/ui/input";
import { Label } from "../components/ui/label";

export function Profiles() {
  const { t } = useI18n();
  const {
    cfg,
    editing,
    runningProfile,
    addProfile,
    removeProfile,
    selectProfile,
    reload,
    binary,
  } = useStore();
  const { toast } = useToast();

  const [createOpen, setCreateOpen] = useState(false);
  const [name, setName] = useState("");
  const [deleteTarget, setDeleteTarget] = useState<string | null>(null);

  const profiles = cfg?.profiles ?? [];

  const create = async () => {
    const trimmed = name.trim();
    if (!trimmed) return;
    await addProfile(trimmed);
    setName("");
    setCreateOpen(false);
  };

  const doExport = async (masked: boolean) => {
    const path = await save({
      title: t("action.export"),
      defaultPath: masked ? "perch-config-masked.toml" : "perch-config.toml",
      filters: [{ name: "TOML", extensions: ["toml"] }],
    });
    if (!path) return;
    try {
      await api.exportConfig(path, masked);
      toast(t("action.saved"), "success");
    } catch (e) {
      toast(String(e), "error");
    }
  };

  const doImport = async () => {
    const path = await open({
      multiple: false,
      directory: false,
      filters: [{ name: "TOML", extensions: ["toml"] }],
    });
    if (typeof path !== "string") return;
    try {
      await api.importConfig(path);
      await reload();
      toast(t("profiles.importNeedModel"), "warning");
    } catch (e) {
      toast(String(e), "error");
    }
  };

  return (
    <div className="animate-fade-in p-6">
      <div className="mb-5 flex items-start justify-between gap-4">
        <div>
          <h1 className="text-lg font-semibold">{t("profiles.title")}</h1>
          <p className="mt-0.5 text-xs text-muted-foreground">{t("profiles.subtitle")}</p>
        </div>
        <div className="flex items-center gap-2">
          <Button variant="outline" size="sm" onClick={() => void doImport()}>
            <Upload className="h-3.5 w-3.5" />
            {t("action.import")}
          </Button>
          <Button variant="outline" size="sm" onClick={() => void doExport(true)}>
            <Download className="h-3.5 w-3.5" />
            {t("profiles.exportMasked")}
          </Button>
          <Button variant="outline" size="sm" onClick={() => void doExport(false)}>
            <FileDown className="h-3.5 w-3.5" />
            {t("profiles.exportFull")}
          </Button>
          <Button size="sm" onClick={() => setCreateOpen(true)}>
            <Plus className="h-3.5 w-3.5" />
            {t("profiles.new")}
          </Button>
        </div>
      </div>

      <div className="mb-4 flex flex-wrap items-center gap-3 text-xs text-muted-foreground">
        <span>{t("profiles.count", { n: profiles.length })}</span>
        <span className="text-border">|</span>
        <span>{t("profiles.switchHint")}</span>
        {profiles.length > 50 && (
          <Badge tone="warning">{t("profiles.softLimit", { n: 50 })}</Badge>
        )}
        {binary && <Badge tone="muted">{binary.backend}</Badge>}
      </div>

      {profiles.length === 0 ? (
        <Card>
          <CardContent className="flex flex-col items-center gap-2 py-14 text-muted-foreground">
            <FolderCog className="h-8 w-8" />
            <p className="text-sm">{t("profiles.empty")}</p>
          </CardContent>
        </Card>
      ) : (
        <div className="grid gap-3 md:grid-cols-2 2xl:grid-cols-3">
          {profiles.map((p) => {
            const isEditing = p.name === editing;
            const isRunning = p.name === runningProfile;
            const isLast = p.name === cfg?.global.last_profile;
            return (
              <Card
                key={p.name}
                onClick={() => void selectProfile(p.name)}
                className={cn(
                  "group cursor-pointer transition-all hover:border-primary/40 hover:shadow-md",
                  isEditing && "border-primary/60 ring-1 ring-primary/30",
                )}
              >
                <CardHeader className="flex-row items-start justify-between pb-2">
                  <div className="min-w-0">
                    <CardTitle className="flex items-center gap-2 truncate">
                      {p.name}
                      {isEditing && <Badge tone="default">{t("profiles.editing")}</Badge>}
                      {isRunning && (
                        <Badge tone="success">
                          <Radio className="h-2.5 w-2.5" />
                          {t("profiles.running")}
                        </Badge>
                      )}
                    </CardTitle>
                    <div className="mt-1 truncate text-[11px] text-muted-foreground">
                      {p.model_path ? basename(p.model_path) : t("dashboard.none")}
                    </div>
                  </div>
                  <div className="flex shrink-0 items-center gap-1">
                    {isLast && <CheckCircle2 className="h-3.5 w-3.5 text-success" />}
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        setDeleteTarget(p.name);
                      }}
                      className="rounded p-1 text-muted-foreground opacity-0 transition-all hover:bg-destructive/10 hover:text-destructive group-hover:opacity-100"
                    >
                      <Trash2 className="h-3.5 w-3.5" />
                    </button>
                  </div>
                </CardHeader>
                <CardContent>
                  <div className="mono flex items-center gap-2 text-[11px] text-muted-foreground">
                    <span>
                      {p.host}:{p.port}
                    </span>
                    <span className="text-border">·</span>
                    <span>-ngl {p.ngl < 0 ? "auto" : p.ngl}</span>
                    <span className="text-border">·</span>
                    <span>-c {p.ctx_size || "default"}</span>
                  </div>
                  {p.extra_params.trim() && (
                    <div className="mono mt-1.5 truncate text-[10px] text-muted-foreground/80">
                      {p.extra_params}
                    </div>
                  )}
                </CardContent>
              </Card>
            );
          })}
        </div>
      )}

      <Dialog
        open={createOpen}
        onClose={() => setCreateOpen(false)}
        title={t("profiles.new")}
        description={t("profiles.switchHint")}
        footer={
          <>
            <Button variant="outline" onClick={() => setCreateOpen(false)}>
              {t("action.cancel")}
            </Button>
            <Button onClick={() => void create()} disabled={!name.trim()}>
              {t("action.create")}
            </Button>
          </>
        }
      >
        <div className="space-y-1.5">
          <Label htmlFor="new-profile">{t("profiles.name")}</Label>
          <Input
            id="new-profile"
            autoFocus
            value={name}
            onChange={(e) => setName(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") void create();
            }}
            placeholder={t("profiles.namePlaceholder")}
          />
        </div>
      </Dialog>

      <Dialog
        open={deleteTarget !== null}
        onClose={() => setDeleteTarget(null)}
        title={t("action.delete")}
        description={
          deleteTarget ? t("profiles.deleteConfirm", { name: deleteTarget }) : undefined
        }
        footer={
          <>
            <Button variant="outline" onClick={() => setDeleteTarget(null)}>
              {t("action.cancel")}
            </Button>
            <Button
              variant="destructive"
              onClick={() => {
                if (deleteTarget) void removeProfile(deleteTarget);
                setDeleteTarget(null);
              }}
            >
              {t("action.delete")}
            </Button>
          </>
        }
      />
    </div>
  );
}
