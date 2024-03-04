<script lang="ts">
	import { error } from '../stores/error-store';
</script>

{#if $error}
	<div class="alert alert-error shadow-lg fixed bottom-0 rounded-b-[0px]">
		<div class="flex items-center gap-2">
			<button
				on:click={() => {
					error.set(undefined);
				}}
			>
				<svg
					xmlns="http://www.w3.org/2000/svg"
					class="stroke-current flex-shrink-0 h-6 w-6"
					fill="none"
					viewBox="0 0 24 24"
					><path
						stroke-linecap="round"
						stroke-linejoin="round"
						stroke-width="2"
						d="M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z"
					/></svg
				>
			</button>
			{#if $error === 'No secret found'}
				<div>Secret to databse not found. See: <a 
					class="ttext-red-950 underline"
					target="_blank" href="https://github.com/dwilkolek/wombat/wiki/Configuration#setup-profile-to-access-ssmparameter-store">
				Configuration: Setup profile to access ssm/parameter store</a></div>
			{:else}
				<span>{$error}</span>
			{/if}
		</div>
	</div>
{/if}
