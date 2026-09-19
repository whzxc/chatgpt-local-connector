<script setup lang="ts">
import { SquareArrowOutUpRight } from '@lucide/vue';
import { ref, onMounted, computed, watch } from 'vue';
import { api, useConnector, type Login } from '../composables/useConnector';
import { openUrl } from '../platform';
import CodexAccount from './CodexAccount.vue';
const { status, busy, run, refresh } = useConnector();
const storedStep = Number(localStorage.getItem('onboarding-stage') || 0);
const step = ref([0,1,2].includes(storedStep) ? storedStep : 0);
watch(step,n=>localStorage.setItem('onboarding-stage',String(n)));
const dependencies = ref<any>();
const tunnelId = ref(status.value?.config.tunnelId || '');
const apiKey = ref('');
const ready = computed(()=>status.value?.core.desktop?.supported && dependencies.value?.tunnel?.path);
const guide = 'https://developers.openai.com/api/docs/guides/secure-mcp-tunnels';
async function inspect() { dependencies.value=await api('dependencies'); }
async function install() { if(!dependencies.value?.tunnel?.path) await api('install','POST',{component:'tunnel'}); await inspect(); await refresh(); }
async function next() {
  await run('setup',async()=>{
    if(step.value===0){await inspect();if(!ready.value)throw new Error('请先准备运行组件');step.value=1;}
    else if(step.value===1){const login=await api<Login>('codex/login');if(login.state!=='authenticated')throw new Error('请先完成 Codex 授权，再继续');step.value=2;}
    else {
      if(!/^tunnel_[A-Za-z0-9_-]+$/.test(tunnelId.value.trim()))throw new Error('请输入以 tunnel_ 开头的 Tunnel ID');
      if(!apiKey.value.trim()&&!status.value?.config.hasApiKey)throw new Error('请输入 runtime API Key');
      await api('config','PUT',{tunnelBinary:status.value!.config.tunnelBinary,codexBinary:status.value!.config.codexBinary,autoStart:status.value!.config.autoStart,tunnelId:tunnelId.value.trim(),apiKey:apiKey.value.trim()});
      apiKey.value='';localStorage.removeItem('onboarding-stage');await refresh();
    }
  });
}
onMounted(()=>run('setup-inspect',async()=>{await inspect();if(!ready.value)step.value=0;else if(step.value>0&&(await api<Login>('codex/login')).state!=='authenticated')step.value=1;}));
</script>
<template>
  <section class="focus-page onboarding">
    <div class="step-dots" aria-label="设置进度"><i v-for="n in 3" :class="{active:n===step+1,complete:n<step+1}"></i><span>{{step+1}} / 3</span></div>
    <Transition name="step" mode="out-in"><div :key="step">
      <div class="eyebrow">LET’S GET CONNECTED</div>
      <h1>{{['先，为连接做好准备。','让 Codex 认识你。','连接你的本机通道。'][step]}}</h1>
      <p class="page-subtitle">{{['我们会检查这台电脑，并补齐需要的组件。','使用你的 OpenAI 账号，在官方页面完成授权。','填入通道信息。开启连接后，我们会引导你添加 ChatGPT 插件。'][step]}}</p>
      <template v-if="step===0"><div class="dependency-row"><span>Codex Desktop</span><span>{{status?.core.desktop?.supported?'已安装':'需要安装（macOS）'}}</span></div><button v-if="!status?.core.desktop?.supported" class="text-button" @click="run('desktop-download',()=>openUrl('https://chatgpt.com/codex'))">获取 Codex Desktop <SquareArrowOutUpRight class="external-icon" aria-hidden="true" /></button><div class="dependency-row"><span>Tunnel Client</span><span>{{!dependencies?'检查中…':dependencies.tunnel?.path?'已就绪':'需要安装'}}</span></div><button v-if="dependencies&&!dependencies.tunnel?.path" class="primary" :disabled="!!busy" @click="run('install-setup',install)">{{busy?'正在准备…':'安装所需组件'}}</button></template>
      <CodexAccount v-else-if="step===1"/>
      <form v-else @submit.prevent="next"><label for="setup-tunnel">Tunnel ID</label><input id="setup-tunnel" v-model="tunnelId" placeholder="tunnel_…" autocomplete="off" required/><label for="setup-key">Runtime API Key</label><input id="setup-key" v-model="apiKey" type="password" placeholder="粘贴你的运行密钥" autocomplete="new-password" :required="!status?.config.hasApiKey"/><details class="credential-help" open><summary>从哪里获取？</summary><p>在 OpenAI Platform 的 Tunnel 设置中创建通道，并获取 runtime API Key。没有权限时，请向组织管理员索取；同时确认通道已关联目标 ChatGPT 工作区。</p><button type="button" class="text-button" @click="run('tunnels',()=>openUrl('https://platform.openai.com/settings/organization/tunnels'))">前往 Tunnel 设置 <SquareArrowOutUpRight class="external-icon" aria-hidden="true" /></button><br/><button type="button" class="text-button" @click="run('guide',()=>openUrl(guide))">查看官方操作指南 <SquareArrowOutUpRight class="external-icon" aria-hidden="true" /></button></details></form>
      <div class="wizard-actions"><button class="text-button" :disabled="step===0||!!busy" @click="step--">← 上一步</button><button class="primary" :disabled="!!busy||(step===0&&!ready)" @click="next">{{busy?'请稍候…':step===2?'保存，前往连接':'继续'}} <span>→</span></button></div>
    </div></Transition>
  </section>
</template>
