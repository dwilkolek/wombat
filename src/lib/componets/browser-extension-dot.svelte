<script lang="ts">
	import { browserExtensionStatus } from '$lib/stores/browser-extension-status';
	import { BrowserExtensionState } from '$lib/types';
</script>

{#if $browserExtensionStatus.state == BrowserExtensionState.Disconnected}
	<div
		class="bg-rose-500 w-2 h-2 rounded opacity-50 tooltip tooltip-right"
		data-tip="Disconnected"
	/>
{:else if $browserExtensionStatus.state == BrowserExtensionState.NotSupported}
	<div
		class="bg-rose-500 w-2 h-2 rounded tooltip tooltip-right"
		data-tip={`${$browserExtensionStatus.version} is not supported.`}
	/>
{:else if $browserExtensionStatus.state == BrowserExtensionState.Outdated}
	<div
		class="bg-amber-500 w-2 h-2 rounded tooltip tooltip-right"
		data-tip={`${$browserExtensionStatus.version} is outdated.`}
	/>
{:else}
	<div
		class="bg-lime-500 w-2 h-2 rounded tooltip tooltip-right"
		data-tip={`${$browserExtensionStatus.version}.`}
	/>
{/if}
