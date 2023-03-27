<script lang="ts">
	import { Environment, state } from "../store";
	let records = state.records;
  const environments = Object.keys(Environment).map(
    (env) => env as Environment
  );
  const env = state.env;
  const profile = state.profile;

  </script>

<svelte:head>
	<title>Home</title>
	<meta name="description" content="Wombat" />
</svelte:head>

<div class="navbar bg-base-100 ">
	<input
	  type="text"
	  disabled="{true}"
	  placeholder="AWS profile"
	  class="input input-bordered w-full max-w-xs text-accent-content"
	  bind:value={$profile}
	/>
	<div class="dropdown dropdown-end">
	  <label tabindex="0" class="btn m-1 text-accent-content">{$env}</label>
	  <ul
		tabindex="0"
		class="dropdown-content menu p-2 shadow bg-base-100 rounded-box w-52"
	  >
		{#each environments as env}
		  <li><a on:click={() => state.selectEnvironment(env)}>{env}</a></li>
		{/each}
	  </ul>
	</div>
  
  
  </div>
  
	<div class="mx-auto">
	  <div class="overflow-x-auto">
		<table class="table w-full">
		  <!-- head -->
		  <thead>
			<tr>
			  <th />
			  <th>Service name</th>
			  <th>DBs</th>
			</tr>
		  </thead>
		  <tbody>
			{#each $records as record, i}
			  <tr>
				<th>{i}</th>
				<td>{record.service}</td>
				<td>
				  {#each record.dbs as db}
					<ul>
					  <li>{db.db_name}</li>
					</ul>
				  {/each}
				</td>
			  </tr>
			{/each}
		  </tbody>
		</table>
	  </div>
	</div>
