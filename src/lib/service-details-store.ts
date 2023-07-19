import { derived, writable } from 'svelte/store';
import { invoke } from '@tauri-apps/api';
import { listen } from '@tauri-apps/api/event';
import type { AwsEnv, DbInstance, ServiceDetails } from '$lib/types';
import { ENVIRONMENTS } from './env-store';
type ServiceDetailsPayload = {
	app: string;
	dbs: DbInstance[];
	services: ServiceDetails[];
};
const createServiceDetailsStore = () => {
	const innerStore = writable<ServiceDetailsPayload[]>([]);
	listen<Omit<ServiceDetailsPayload, 'loading'>>('new-service-details', (data) => {
		console.log('new-service-details', data);
		innerStore.update((old) => {
			return [...old.filter((o) => o.app !== data.payload['app']), { ...data.payload }].sort(
				(a, b) => a.app.localeCompare(b.app)
			);
		});
	});
	setInterval(
		() => {
			innerStore.subscribe((apps) => {
				apps.forEach((app) => {
					invoke('service_details', { app: app.app });
				});
			});
		},
		5 * 60 * 1000
	);
	return { ...innerStore };
};

const askedForStore = writable<string[]>([]);

listen('cache-refreshed', () => {
	askedForStore.set([]);
	serviceDetailsStore.set([]);
});

const serviceDetailsStore = createServiceDetailsStore();
export const serviceDetailStore = (app: string) =>
	derived([serviceDetailsStore, askedForStore], (stores) => {
		const apps = stores[0];
		const askedFor = stores[1];
		const details = apps.find((a) => a.app === app);
		if (!details) {
			if (!askedFor.includes(app)) {
				askedForStore.update((data) => {
					setTimeout(() => {
						invoke('service_details', { app });
					}, 200);
					return [...data, app];
				});
			}
			return undefined;
		}
		if (details) {
			const result = {
				app,
				envs: new Map<AwsEnv, { services: ServiceDetails[]; dbs: DbInstance[] }>()
			};
			for (const env of ENVIRONMENTS) {
				const services = details.services.filter((s) => s.env === env);
				const dbs = details.dbs.filter((s) => s.env === env);
				if (services.length || dbs.length) {
					result.envs.set(env, {
						services,
						dbs
					});
				}
			}

			return result;
		}
		return details;
	});
