import { get, writable } from 'svelte/store';
import { listen } from '@tauri-apps/api/event';
import type { Cluster, EcsService } from '$lib/types';
import { invoke } from '@tauri-apps/api';
import { clusterStore } from './cluster-store';

const createServiceStore = () => {
	const innerStore = writable(new Map<Cluster, EcsService[]>());
	const selectedService = writable<EcsService | null>(null);
	const selectedServices = writable<EcsService[]>([]);
	const getServices = async (cluster?: Cluster): Promise<EcsService[]> => {
		if (!cluster) {
			return [];
		}
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

	const selectService = (selection: EcsService) => {
		selectedService.set(selection);
		selectedServices.update((services) => {
			let newSelection;
			if (services.includes(selection)) {
				newSelection = services.filter((s) => s.arn != selection.arn);
			} else {
				newSelection = [...services, selection];
			}

			return newSelection;
		});
	};

	clusterStore.activeCluser.subscribe((activeCluster) => {
		getServices(activeCluster).then((servicesInNewCluster) => {
			const service = get(selectedService);
			if (service) {
				if (service.cluster_arn != activeCluster.arn) {
					const serviceFromNewCluster = servicesInNewCluster.find((s) => s.name === service.name);
					selectedService.set(serviceFromNewCluster ?? null);
				}
			}

			const newSelectedServices = get(selectedServices)
				.map((service) => servicesInNewCluster.find((s) => s.name === service.name))
				.filter((o) => !!o) as EcsService[];
			selectedServices.set(newSelectedServices);
		});
	});
	return { ...innerStore, selectedService, selectedServices, getServices, selectService };
};

listen('cache-refreshed', () => {
	serviceStore.set(new Map());
});

listen('logged-out', () => {
	serviceStore.set(new Map());
});
export const serviceStore = createServiceStore();
