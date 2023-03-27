<script lang="ts">
	import { goto } from '$app/navigation';
	import { state } from "./store";
	let storeProfile = state.profile;
	let storeErr = state.error;
	let inputProfile = "developer";
	state.profile.subscribe((profile) => {
	  inputProfile = profile ?? "developer";
	});
	let loading = false;
  </script>

<svelte:head>
	<title>Home</title>
	<meta name="description" content="Wombat" />
</svelte:head>

  
<div class="hero min-h-screen bg-base-200">
	<div class="hero-content text-center">
	  <div class="max-w-lg">
		<h1 class="text-5xl font-bold">Hello friendly wombat!</h1>
		<input
	  type="text"
	  placeholder="AWS profile"
	  class="input input-bordered w-full max-w-xs text-accent-content"
	  bind:value={inputProfile}
	/>
	<button
			class="btn btn-accent"
			disabled={loading || inputProfile == $storeProfile}
			on:click={async () => {
			loading = true;
			console.log(inputProfile, loading, storeProfile);
			await state.start(inputProfile);
			loading = false;
			goto(`/matched-entries`, { replaceState: true }) 
			}}
		>
			Get Start</button
		>
		{$storeErr ?? ""}
	  </div>
	</div>
  </div>
