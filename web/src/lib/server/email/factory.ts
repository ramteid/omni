import type { EmailProvider, EmailConfig } from './types'
import { ResendEmailProvider } from './providers/resend'
import { SMTPEmailProvider } from './providers/smtp'
import {
    EMAIL_PROVIDER,
    RESEND_API_KEY,
    EMAIL_FROM,
    EMAIL_HOST,
    EMAIL_PORT,
    EMAIL_USER,
    EMAIL_PASSWORD,
    EMAIL_SECURE,
} from '$env/static/private'

let emailProvider: EmailProvider | null = null

export function getEmailProvider(): EmailProvider {
    if (emailProvider) {
        return emailProvider
    }

    const config: EmailConfig = {
        provider: (EMAIL_PROVIDER as 'resend' | 'smtp') || 'resend',
        resendApiKey: RESEND_API_KEY || undefined,
        fromEmail: EMAIL_FROM || 'Clio <noreply@yourdomain.com>',
        smtpHost: EMAIL_HOST || undefined,
        smtpPort: EMAIL_PORT ? parseInt(EMAIL_PORT) : undefined,
        smtpUser: EMAIL_USER || undefined,
        smtpPassword: EMAIL_PASSWORD || undefined,
        smtpSecure: EMAIL_SECURE === 'true',
    }

    if (config.provider === 'resend') {
        if (!config.resendApiKey) {
            throw new Error(
                'RESEND_API_KEY environment variable is required when using Resend provider',
            )
        }
        emailProvider = new ResendEmailProvider(config.resendApiKey, config.fromEmail)
    } else if (config.provider === 'smtp') {
        if (!config.smtpHost || !config.smtpUser || !config.smtpPassword) {
            throw new Error(
                'SMTP configuration (EMAIL_HOST, EMAIL_USER, EMAIL_PASSWORD) is required when using SMTP provider',
            )
        }
        emailProvider = new SMTPEmailProvider(config)
    } else {
        throw new Error(`Unsupported email provider: ${config.provider}`)
    }

    return emailProvider
}

export function resetEmailProvider(): void {
    emailProvider = null
}
