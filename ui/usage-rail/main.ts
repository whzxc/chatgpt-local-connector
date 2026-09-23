import { createApp,h } from 'vue';
import { i18n } from '../i18n';
import UiProvider from '../components/UiProvider.vue';
import './window.css';
import UsageRail from './UsageRail.vue';
createApp({render:()=>h(UiProvider,{forceDark:true},{default:()=>h(UsageRail)})}).use(i18n).mount('#app');
