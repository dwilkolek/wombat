import { writable } from 'svelte/store';
import { listen } from '@tauri-apps/api/event';
import { triggerSilentServiceDetailsRefresh } from './service-details-store';

export type DeploymentRolloutStatus = {
	deployment_id: string;
	service_name: string;
	cluster_arn: string;
	rollout_status: 'Unknown' | 'Completed' | 'Failed' | 'In Progress';
	version: string | undefined;
	error_message: string | undefined;
};
const createDeploymentStore = () => {
	const deplyoments = writable<DeploymentRolloutStatus[]>([]);
	listen<DeploymentRolloutStatus>('deployment', (event) => {
		console.log(
			'deployment update: ',
			event.payload.rollout_status,
			event.payload.rollout_status in ['Failed', 'Completed'],
			event
		);
		console.log('refresh?', ['Failed', 'Completed'].includes(event.payload.rollout_status));
		if (['Failed', 'Completed'].includes(event.payload.rollout_status)) {
			console.log('Triggering refresh', event.payload.service_name);
			triggerSilentServiceDetailsRefresh(event.payload.service_name);
		}
		deplyoments.update((deployments) => {
			return [
				...deployments.filter(
					(deployment) => deployment.deployment_id != event.payload.deployment_id
				),
				event.payload
			];
		});
	});

	return {
		...deplyoments,
		clear: (deploymentId: string) => {
			deplyoments.update((deplyoments) => {
				return deplyoments.filter((d) => d.deployment_id != deploymentId);
			});
		}
	};
};
export const deplyomentStore = createDeploymentStore();
