import { useEffect, useState } from "react";
import { t } from "../i18n";
import { subscriptions, saveSubscriptions } from "../state/subscriptions";
import {
  quotaDisplay,
  resetDisplay,
  usagePeriods,
  usagePeriodLabels,
  maxVisibleAgents,
} from "../subscriptions/displayPreferences";
import {
  panelPreferences,
  readPanelPreferences,
  savePanelPreferences,
} from "../usage-rail/preferences";
import type { PanelPreferences } from "../usage-rail/layout";
import {
  Checkbox,
  Dialog,
  Group,
  Row,
  Notice,
  SingleChoice,
  Switch,
} from "./ui";
export default function AgentDisplaySettings({
  onClose,
}: {
  onClose: () => void;
}) {
  const { snapshot } = subscriptions.use(),
    prefs = panelPreferences.use(),
    quota = quotaDisplay.use(),
    reset = resetDisplay.use(),
    periods = usagePeriods.use(),
    max = maxVisibleAgents.use();
  const [saving, setSaving] = useState(false),
    [error, setError] = useState("");
  useEffect(() => {
    void readPanelPreferences().catch((e) => setError(String(e)));
  }, []);
  async function save(action: () => Promise<unknown>) {
    setSaving(true);
    setError("");
    try {
      await action();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  }
  function appearance(patch: Partial<PanelPreferences>) {
    void save(async () => {
      if (patch.visible && snapshot)
        await saveSubscriptions({
          enabled: true,
          providers: snapshot.providers.map((p) => p.providerId),
        });
      await savePanelPreferences(patch);
    });
  }
  return (
    <Dialog title={t("agentDisplaySettings")} onClose={onClose} busy={saving}>
      {error && <Notice>{error}</Notice>}
      <Group title={t("usageAppearance")}>
        {prefs.visible !== false &&
          snapshot &&
          !snapshot.providers.some((p) => p.eligible) && (
            <Notice tone="warning">{t("usageNoAvailableProviders")}</Notice>
          )}
        <Row title={t("usageShow")}>
          <Switch
            label={t("usageShow")}
            checked={
              prefs.visible !== false &&
              !!snapshot?.settings.enabled &&
              snapshot.settings.providers.length > 0
            }
            disabled={saving || !snapshot}
            onChange={(visible) => appearance({ visible })}
          />
        </Row>
        <Row title={t("usageSize")}>
          <SingleChoice
            value={prefs.size}
            label={t("usageSize")}
            disabled={saving}
            options={["small", "standard", "large"].map((value) => ({
              value,
              label: t(
                {
                  small: "usageSmall",
                  standard: "usageStandard",
                  large: "usageLarge",
                }[value]!,
              ),
            }))}
            onChange={(size) =>
              appearance({ size: size as PanelPreferences["size"] })
            }
          />
        </Row>
        <Row title={t("usageSpacing")}>
          <SingleChoice
            value={prefs.spacing}
            label={t("usageSpacing")}
            disabled={saving}
            options={["compact", "standard", "roomy"].map((value) => ({
              value,
              label: t(
                {
                  compact: "usageCompact",
                  standard: "usageStandard",
                  roomy: "usageRoomy",
                }[value]!,
              ),
            }))}
            onChange={(spacing) =>
              appearance({ spacing: spacing as PanelPreferences["spacing"] })
            }
          />
        </Row>
        <Row title={t("agentsMaxVisible")}>
          <SingleChoice
            value={String(max)}
            onChange={(value) => maxVisibleAgents.set(Number(value))}
            label={t("agentsMaxVisible")}
            options={["4", "6", "8", "10"].map((value) => ({
              value,
              label: value,
            }))}
          />
        </Row>
      </Group>
      <Group title={t("agentQuotaGroup")}>
        <Row title={t("agentQuotaDisplay")}>
          <SingleChoice
            value={quota}
            onChange={(value) =>
              quotaDisplay.set(value as "remaining" | "used")
            }
            label={t("agentQuotaDisplay")}
            options={[
              { value: "remaining", label: t("usageRemaining") },
              { value: "used", label: t("usageUsed") },
            ]}
          />
        </Row>
        <Row title={t("agentResetDisplay")}>
          <SingleChoice
            value={reset}
            onChange={(value) =>
              resetDisplay.set(value as "countdown" | "time")
            }
            label={t("agentResetDisplay")}
            options={[
              { value: "countdown", label: t("usageCountdown") },
              { value: "time", label: t("usageResetTime") },
            ]}
          />
        </Row>
        <Row title={t("usageAutoRefresh")}>
          <SingleChoice
            value={String(snapshot?.settings.refreshMinutes ?? 3)}
            onChange={(value) =>
              void save(() =>
                saveSubscriptions({ refreshMinutes: Number(value) }),
              )
            }
            label={t("usageAutoRefresh")}
            disabled={saving || !snapshot}
            options={["1", "3", "5", "10"].map((value) => ({
              value,
              label: `${value} min`,
            }))}
          />
        </Row>
      </Group>
      <Group title={t("usageGroup")}>
        <Row title={t("usageDuration")}>
          <div className="period-options">
            {Object.entries(usagePeriodLabels).map(([id, label]) => (
              <Checkbox
                key={id}
                checked={periods.includes(id)}
                onChange={(checked) =>
                  usagePeriods.set(
                    checked
                      ? [...periods, id]
                      : periods.filter((p) => p !== id),
                  )
                }
              >
                {t(label)}
              </Checkbox>
            ))}
          </div>
        </Row>
      </Group>
    </Dialog>
  );
}
