import { useStorage } from '@vueuse/core';

export const controlSourceClick = useStorage<'service' | 'settings'>('controlSourceClick', 'service', undefined, {
  serializer: { read: value => value === 'settings' ? 'settings' : 'service', write: value => value },
});
