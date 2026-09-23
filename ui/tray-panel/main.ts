import {createApp,h} from 'vue';
import {i18n} from '../i18n';
import UiProvider from '../components/UiProvider.vue';
import TrayPanel from './TrayPanel.vue';
import '../style.css';
import './window.css';
createApp({render:()=>h(UiProvider,null,{default:()=>h(TrayPanel)})}).use(i18n).mount('#app');
