<script lang="ts">
	import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '$lib/components/ui/card/index.js';
	import { Button } from '$lib/components/ui/button/index.js';

	async function logout() {
		await fetch('/logout', {
			method: 'POST'
		});
		window.location.href = '/login';
	}
</script>

<svelte:head>
	<title>Account Pending - Clio</title>
</svelte:head>

<div class="min-h-screen bg-gradient-to-br from-background to-muted">
	<div class="flex min-h-screen items-center justify-center p-4">
		<div class="w-full max-w-md">
			<div class="mb-8 text-center">
				<h1 class="text-3xl font-bold text-foreground">Clio</h1>
				<p class="mt-2 text-muted-foreground">Enterprise Search Platform</p>
			</div>
			
			<Card class="w-full">
				<CardHeader class="text-center">
					<CardTitle class="text-2xl">Account Pending</CardTitle>
					<CardDescription>Your account is awaiting approval</CardDescription>
				</CardHeader>
				<CardContent class="text-center space-y-4">
					<div class="rounded-md bg-yellow-50 p-4 dark:bg-yellow-900/50">
						<div class="text-sm text-yellow-800 dark:text-yellow-200">
							Your account has been created successfully, but it needs to be approved by an administrator before you can access Clio.
						</div>
					</div>
					
					<p class="text-sm text-muted-foreground">
						You will receive an email notification once your account has been approved. Please contact your administrator if you have any questions.
					</p>
					
					<Button on:click={logout} class="w-full">
						Sign out
					</Button>
				</CardContent>
			</Card>
		</div>
	</div>
</div>