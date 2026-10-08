<script lang="ts">
	import { featuresStore } from '#lib/stores/feature-store.js';
	import type { ServiceDetails } from '#lib/types.js';
	import DeployServiceBtn from './deploy-service-btn.svelte';
	import RestartServiceBtn from './restart-service-btn.svelte';

	interface Props {
		service: ServiceDetails;
	}

	let { service }: Props = $props();
</script>

{#if $featuresStore.deployEcsService}
	<DeployServiceBtn {service} />
{:else}
	<RestartServiceBtn {service} />
{/if}
