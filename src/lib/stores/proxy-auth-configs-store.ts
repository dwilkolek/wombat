import type { ProxyAuthConfig } from "$lib/types";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { writable } from "svelte/store";



const createProxyAuthConfigsStore = () => {
    const configsStore = writable<ProxyAuthConfig[]>([]);
	invoke<ProxyAuthConfig[]>('proxy_auth_configs').then((configs) => {
        configsStore.set(configs);
    });
    listen('cache-refreshed', () => {
        invoke<ProxyAuthConfig[]>('proxy_auth_configs').then((configs) => {
            configsStore.set(configs);
        });
    });
	return { ...configsStore };
};
export const proxyAuthConfigsStore = createProxyAuthConfigsStore();