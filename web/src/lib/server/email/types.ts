export interface EmailResult {
    success: boolean
    messageId?: string
    error?: string
}

export interface EmailProvider {
    sendMagicLink(email: string, magicLinkUrl: string, isNewUser?: boolean): Promise<EmailResult>

    testConnection(): Promise<boolean>
}

export interface EmailConfig {
    provider: 'resend' | 'smtp'
    // Resend config
    resendApiKey?: string
    // SMTP config
    smtpHost?: string
    smtpPort?: number
    smtpUser?: string
    smtpPassword?: string
    smtpSecure?: boolean
    // Common config
    fromEmail: string
}
