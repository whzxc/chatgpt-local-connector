import "./style.css";
import { createRoot } from "react-dom/client";
import UiProvider from "./components/UiProvider";
import App from "./App";
import "./product.css";
createRoot(document.getElementById("app")!).render(
  <UiProvider>
    <App />
  </UiProvider>,
);
