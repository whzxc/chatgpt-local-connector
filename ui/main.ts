import { createApp } from "vue";
import App from "./App.vue";
import "./style.css";

createApp(App).mount("#app");

document.documentElement.dataset.theme = localStorage.getItem('theme') || 'system';
