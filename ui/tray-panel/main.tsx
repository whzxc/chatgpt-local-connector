import "../style.css";
import { useEffect } from "react";
import { createRoot } from "react-dom/client";
import UiProvider from "../components/UiProvider";
import { startSubscriptions } from "../state/subscriptions";
import TrayPanel from "./TrayPanel";
import "./window.css";
function Root() {
  useEffect(startSubscriptions, []);
  return <TrayPanel />;
}
createRoot(document.getElementById("app")!).render(
  <UiProvider>
    <Root />
  </UiProvider>,
);
