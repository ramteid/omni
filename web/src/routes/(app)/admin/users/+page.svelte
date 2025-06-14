<script lang="ts">
	import type { PageData } from './$types';
	import { enhance } from '$app/forms';

	export let data: PageData;

	function formatDate(date: Date | null) {
		if (!date) return 'N/A';
		return new Date(date).toLocaleDateString();
	}
</script>

<div class="min-h-screen bg-background">
	<nav class="bg-card shadow border-b">
		<div class="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
			<div class="flex h-16 justify-between">
				<div class="flex items-center">
					<h1 class="text-xl font-semibold">Clio Admin - User Management</h1>
				</div>
				<div class="flex items-center space-x-4">
					<a href="/" class="text-sm text-muted-foreground hover:text-foreground">Back to Home</a>
				</div>
			</div>
		</div>
	</nav>

	<main class="mx-auto max-w-7xl px-4 py-8 sm:px-6 lg:px-8">
		<div class="rounded-lg bg-card shadow border">
			<div class="px-4 py-5 sm:p-6">
				<h2 class="text-lg font-medium text-foreground">Users</h2>
				
				<div class="mt-6 overflow-hidden shadow ring-1 ring-border md:rounded-lg">
					<table class="min-w-full divide-y divide-border">
						<thead class="bg-muted/50">
							<tr>
								<th class="px-6 py-3 text-left text-sm font-semibold text-foreground">Email</th>
								<th class="px-6 py-3 text-left text-sm font-semibold text-foreground">Role</th>
								<th class="px-6 py-3 text-left text-sm font-semibold text-foreground">Status</th>
								<th class="px-6 py-3 text-left text-sm font-semibold text-foreground">Created</th>
								<th class="px-6 py-3 text-left text-sm font-semibold text-foreground">Updated</th>
								<th class="relative px-6 py-3"><span class="sr-only">Actions</span></th>
							</tr>
						</thead>
						<tbody class="divide-y divide-border bg-background">
							{#each data.users as user}
								<tr>
									<td class="px-6 py-4 text-sm font-medium text-foreground">
										{user.email}
									</td>
									<td class="px-6 py-4 text-sm text-muted-foreground">
										<form method="POST" action="?/updateRole" use:enhance class="inline">
											<input type="hidden" name="userId" value={user.id} />
											<select
												name="role"
												value={user.role}
												on:change={(e) => e.currentTarget.form?.requestSubmit()}
												class="rounded-md border-border text-sm bg-background"
											>
												<option value="admin">Admin</option>
												<option value="user">User</option>
												<option value="viewer">Viewer</option>
											</select>
										</form>
									</td>
									<td class="px-6 py-4 text-sm">
										<span class="inline-flex rounded-full px-2 text-xs font-semibold leading-5
											{user.isActive ? 'bg-green-100 text-green-800 dark:bg-green-900/20 dark:text-green-400' : 'bg-destructive/10 text-destructive'}
										">
											{user.isActive ? 'Active' : 'Inactive'}
										</span>
									</td>
									<td class="px-6 py-4 text-sm text-muted-foreground">
										{formatDate(user.createdAt)}
									</td>
									<td class="px-6 py-4 text-sm text-muted-foreground">
										{formatDate(user.updatedAt)}
									</td>
									<td class="px-6 py-4 text-right text-sm font-medium">
										<div class="flex justify-end space-x-2">
											{#if user.isActive}
												<form method="POST" action="?/suspend" use:enhance>
													<input type="hidden" name="userId" value={user.id} />
													<button
														type="submit"
														class="text-destructive hover:text-destructive/80"
													>
														Deactivate
													</button>
												</form>
											{:else}
												<form method="POST" action="?/activate" use:enhance>
													<input type="hidden" name="userId" value={user.id} />
													<button
														type="submit"
														class="text-green-600 hover:text-green-900 dark:text-green-400 dark:hover:text-green-300"
													>
														Activate
													</button>
												</form>
											{/if}
										</div>
									</td>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>
			</div>
		</div>
	</main>
</div>