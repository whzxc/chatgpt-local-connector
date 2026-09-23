import {createApp,h} from 'vue';
import {i18n} from '../i18n';
import UiProvider from '../components/UiProvider.vue';
import TrayDetail from './TrayDetail.vue';
import '../style.css';
import './window.css';
createApp({render:()=>h(UiProvider,null,{default:()=>h(TrayDetail)})}).use(i18n).mount('#app');
