import { get, writable } from 'svelte/store';
import { listen } from '@tauri-apps/api/event';
import type { Cluster, EcsService } from '$lib/types';
import { invoke } from '@tauri-apps/api';

const createServiceStore = () => {
	const innerStore = writable(new Map<Cluster, EcsService[]>());
	const selectedService = writable<EcsService | null>(null);
	const getServices = async (cluster: Cluster): Promise<EcsService[]> => {
		if (get(innerStore).has(cluster)) {
			return get(innerStore).get(cluster)!;
		} else {
			const services = await invoke<EcsService[]>('services', { cluster });
			const clone = new Map(get(innerStore));
			clone.set(cluster, services);
			innerStore.set(clone);
			return services;
		}
	};
	const selectService = (service: EcsService) => {
		selectedService.set(service)
	}
	return { ...innerStore,selectedService, getServices, selectService };
};

listen('cache-refreshed', () => {
	serviceStore.set(new Map());
});

listen('logged-out', () => {
	serviceStore.set(new Map());
});
export const serviceStore = createServiceStore();
