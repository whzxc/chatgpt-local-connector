import "../style.css";
import { useEffect } from "react";
import { createRoot } from "react-dom/client";
import UiProvider from "../components/UiProvider";
import { locale } from "../i18n";
import { startAgentActivity, startSubscriptions } from "../state/subscriptions";
import UsageRail from "./UsageRail";
import "./window.css";
function Root() {
  locale.use();
  useEffect(() => {
    const stop = startSubscriptions(),
      activity = startAgentActivity();
    return () => {
      stop();
      activity();
    };
  }, []);
  return <UsageRail />;
}
createRoot(document.getElementById("app")!).render(
  <UiProvider>
    <Root />
  </UiProvider>,
);
