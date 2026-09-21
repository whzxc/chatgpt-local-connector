import { createApp, h } from "vue";
import { i18n } from "./i18n";
import UiProvider from "./components/UiProvider.vue";
import App from "./App.vue";
import "./style.css";

createApp({ render: () => h(UiProvider, null, { default: () => h(App) }) }).use(i18n).mount("#app");
