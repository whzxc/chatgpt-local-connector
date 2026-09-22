import { h, type FunctionalComponent } from 'vue';
import { Globe } from '@lucide/vue';
import ngrok from './ngrok.png';
import pinggy from './pinggy.png';
import localxpose from './localxpose.png';
import cloudflare from './cloudflare.svg?raw';

const CloudflareIcon: FunctionalComponent = (_props, { attrs }) => h('svg', { ...attrs, viewBox: '0 0 24 24', fill: 'currentColor', innerHTML: cloudflare.replace(/^<svg[^>]*>|<\/svg>$/g, '') });

export const providerIcons = {
  ngrok: () => h('img', { src: ngrok, alt: '' }),
  cloudflare: CloudflareIcon,
  pinggy: () => h('img', { src: pinggy, alt: '' }),
  localxpose: () => h('img', { src: localxpose, alt: '' }),
  custom: Globe,
};
