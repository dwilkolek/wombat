import { AwsEnv, type EcsService, type RdsInstance } from '$lib/types';
import { derived } from 'svelte/store';
import { wombatProfileStore } from './available-profiles-store';
import { TaskStatus, taskStore } from './task-store';
import { featuresStore } from './feature-store';
import { deplyomentStore } from './deployment-store';

const PROD_ACTIONS_DISABLED_REASON = 'Not available';

export function startEcsProxyDisabledReason(service: EcsService) {
	return derived([featuresStore, taskStore, wombatProfileStore], (stores) => {
		const { devWay, startEcsProxy, ecsProdActions } = stores[0];
		if (stores[1].some((t) => t.arn == service.arn && t.status == TaskStatus.STARTING)) {
			return 'Starting...';
		}
		const matchingInfraProfiles =
			stores[2].infraProfiles.filter((infraProfile) => infraProfile.env == service.env) ?? [];
		if (matchingInfraProfiles.length === 0 && !devWay) {
			return `Missing infra profile: ${service.name}`;
		}
		if (!startEcsProxy) {
			return 'ECS Proxies disabled';
		}
		if (AwsEnv.PROD === service.env && !ecsProdActions) {
			return PROD_ACTIONS_DISABLED_REASON;
		}
	});
}

export function startRdsProxyDisabledReason(rds: RdsInstance) {
	return derived([featuresStore, wombatProfileStore, taskStore], (stores) => {
		const { devWay, startRdsProxy, rdsProdActions } = stores[0];
		if (stores[2].some((t) => t.arn == rds.arn && t.status == TaskStatus.STARTING)) {
			return 'Starting...';
		}
		if (!rds) {
			return 'No RDS selected';
		}
		if (!startRdsProxy) {
			return 'RDS Proxies disabled';
		}
		if (
			!stores[1].infraProfiles.some(
				({ app, env }) => app == rds.normalized_name && env == rds.env
			) &&
			!devWay
		) {
			return `Missing infra profile: ${rds.normalized_name}`;
		}
		if (AwsEnv.PROD === rds.env && !rdsProdActions) {
			return PROD_ACTIONS_DISABLED_REASON;
		}
	});
}

export function startLambdaProxyDisabledReason(lambdaArn: string, env: AwsEnv) {
	return derived(
		[featuresStore, taskStore],
		([{ startLambdaProxy, lambdaProdActions, lambdaApps }, taskStore]) => {
			if (!lambdaApps) {
				return 'Lambda apps disabled';
			}
			if (!startLambdaProxy) {
				return 'Lambda proxy disabled';
			}
			if (taskStore.some((t) => t.arn == lambdaArn && t.status == TaskStatus.STARTING)) {
				return 'Starting...';
			}
			if (AwsEnv.PROD === env && !lambdaProdActions) {
				return PROD_ACTIONS_DISABLED_REASON;
			}
		}
	);
}

export function restartEcsDisabledReason(service: EcsService) {
	return derived(
		[featuresStore, wombatProfileStore, deplyomentStore],
		([{ restartEcsService, ecsProdActions }, wombatProfileStore, deplyomentStore]) => {
			if (!restartEcsService) {
				return { message: 'ECS restart disabled' };
			}
			const missingInfraProfile = !wombatProfileStore.infraProfiles.some(
				({ app, env }) => app == service.name && env == service.env
			);
			if (missingInfraProfile) {
				return { message: `Missing infra profile: ${service.name}` };
			}
			if (AwsEnv.PROD === service.env && !ecsProdActions) {
				return { message: PROD_ACTIONS_DISABLED_REASON };
			}
			const deployment = deplyomentStore.find(
				(deployment) =>
					deployment.service_name == service.name && deployment.cluster_arn == service.cluster_arn
			);
			if (deployment) {
				return { message: 'Deployment in progress', deployment };
			}
		}
	);
}

export function getRdsSecretDisabledReason(rds: RdsInstance | undefined) {
	return derived([featuresStore, wombatProfileStore], (stores) => {
		const { devWay, getRdsSecret, rdsProdActions } = stores[0];
		if (!rds) {
			return 'No RDS selected';
		}
		if (!getRdsSecret) {
			return 'Get RDS secret action disabled';
		}
		if (
			!stores[1].infraProfiles.some(
				({ app, env }) => app == rds.normalized_name && env == rds.env
			) &&
			!devWay
		) {
			return `Missing infra profile: ${rds.normalized_name}`;
		}
		if (AwsEnv.PROD === rds.env && !rdsProdActions) {
			return PROD_ACTIONS_DISABLED_REASON;
		}
	});
}
