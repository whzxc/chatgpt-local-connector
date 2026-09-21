import { createApp } from "vue";
import { i18n } from "./i18n";
import App from "./App.vue";
import "./style.css";

createApp(App).use(i18n).mount("#app");

document.documentElement.dataset.theme = localStorage.getItem('theme') || 'system';
