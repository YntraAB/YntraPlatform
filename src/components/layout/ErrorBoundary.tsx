import React from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { RotateCcw, AlertCircle } from 'lucide-react'
import { useRouteError, isRouteErrorResponse } from 'react-router-dom'

interface ErrorBoundaryProps {
  children: React.ReactNode
  fallback?: React.ReactNode
}

interface ErrorBoundaryState {
  hasError: boolean
  error: Error | null
}

export class ErrorBoundary extends React.Component<ErrorBoundaryProps, ErrorBoundaryState> {
  constructor(props: ErrorBoundaryProps) {
    super(props)
    this.state = { hasError: false, error: null }
  }

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { hasError: true, error }
  }

  componentDidCatch(error: Error, errorInfo: React.ErrorInfo) {
    console.error('ErrorBoundary caught an error:', error, errorInfo)
  }

  render() {
    if (this.state.hasError) {
      if (this.props.fallback) {
        return this.props.fallback
      }
      return <DefaultErrorFallback error={this.state.error} />
    }

    return this.props.children
  }
}

interface FallbackProps {
  error: Error | null
  resetErrorBoundary?: () => void
}

export const DefaultErrorFallback: React.FC<FallbackProps> = ({ error, resetErrorBoundary }) => {
  const { t } = useTranslation()

  const handleRefresh = () => {
    if (resetErrorBoundary) {
      resetErrorBoundary()
    } else {
      window.location.reload()
    }
  }

  return (
    <div className="flex min-h-[360px] w-full flex-1 flex-col items-center justify-center p-6 text-center duration-200 animate-in fade-in">
      <div className="mb-3 flex size-8 items-center justify-center rounded-md border border-border/50 bg-secondary/30 text-muted-foreground/70">
        <AlertCircle className="h-4 w-4" />
      </div>
      <h2 className="mb-1 text-xs font-medium text-foreground tracking-tight">
        {t('common.error_occurred', 'Ett oväntat fel uppstod')}
      </h2>
      <p className="mb-4 max-w-sm text-xs font-normal text-muted-foreground/60 leading-relaxed">
        {t('common.error_unexpected', 'Något gick fel vid inläsning av komponenten.')}
      </p>
      {error?.message && (
        <div className="mb-4 max-w-md rounded-md border border-border/40 bg-secondary/20 px-3 py-2 text-left font-mono text-[11px] text-muted-foreground/80 shadow-none">
          <span className="select-all break-all">{error.message}</span>
        </div>
      )}
      <div className="flex items-center gap-2">
        <Button
          onClick={handleRefresh}
          variant="outline"
          className="h-8 gap-1.5 rounded-md border-border/40 bg-secondary/20 px-3 text-xs font-medium hover:bg-secondary/40"
        >
          <RotateCcw className="h-3 w-3" />
          <span>{t('common.refresh', 'Ladda om')}</span>
        </Button>
        <Button
          onClick={() => {
            window.location.href = '/'
          }}
          variant="ghost"
          className="h-8 rounded-md px-3 text-xs font-normal text-muted-foreground hover:text-foreground"
        >
          <span>{t('common.home', 'Till start')}</span>
        </Button>
      </div>
    </div>
  )
}

export const RouteErrorBoundary: React.FC = () => {
  const error = useRouteError()
  const { t } = useTranslation()

  let errorMessage = t(
    'common.error_unexpected',
    'Ett oväntat fel uppstod. Vänligen försök igen.',
  )

  if (isRouteErrorResponse(error)) {
    errorMessage = `${error.status} ${error.statusText}: ${error.data}`
  } else if (error instanceof Error) {
    errorMessage = error.message
  }

  return (
    <DefaultErrorFallback
      error={new Error(errorMessage)}
      resetErrorBoundary={() => window.location.reload()}
    />
  )
}
