<script setup lang="ts">
import {ref} from 'vue';
import ElasticPanel from '../components/ElasticPanel.vue';
import TrayPanel from './TrayPanel.vue';
import {t} from '../i18n';

type Page='overview'|'settings'|'tasks'|'logs';
const emit=defineEmits<{navigate:[page:Page]}>();
const open=ref(false);
const origin=ref<{x:number;y:number;size:number;height:number}>();
let destination:Page|undefined;
function show(event:MouseEvent){
  const trigger=event.currentTarget as HTMLElement;
  const rect=trigger.getBoundingClientRect();
  origin.value={x:rect.x,y:rect.y,size:rect.width,height:rect.height};
  trigger.focus({preventScroll:true});
  destination=undefined;
  open.value=true;
}
function navigate(page:Page){destination=page;open.value=false;}
function closed(){if(destination){emit('navigate',destination);destination=undefined;}}
defineExpose({show});
</script>

<template>
  <ElasticPanel headerless unframed :show="open" :origin="origin" :title="t('trayUsage')" :width="330" @close="open=false" @closed="closed">
    <TrayPanel embedded @dismiss="open=false" @navigate="navigate"/>
  </ElasticPanel>
</template>
