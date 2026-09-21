import React, { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { ReportForm } from './ReportForm'
import { ReportList } from './ReportList'
import { Shield } from 'lucide-react'

export const ReportingPage: React.FC = () => {
  const { t } = useTranslation()
  const [activeTab, setActiveTab] = useState('send')

  return (
    <div className="relative flex h-full flex-1 flex-col bg-background selection:bg-primary/20">
      {/* Top Header */}
      <div className="sticky top-0 z-20 flex h-12 shrink-0 items-center justify-between border-b border-border/50 bg-background/50 px-6 backdrop-blur-md">
        <h2 className="text-xs font-medium text-foreground">
          {t('reporting.title')}
        </h2>
      </div>

      <div className="scrollbar-dark flex-1 overflow-y-auto p-6 space-y-4 max-w-5xl">
        {/* Trust Banner */}
        <div className="flex items-center gap-3 rounded-md bg-emerald-500/10 p-3.5 border border-emerald-500/20">
          <Shield className="h-4 w-4 text-emerald-500 shrink-0" />
          <p className="text-xs font-medium text-emerald-600/90 dark:text-emerald-400/90">
            Alla anmälningar och visselblåsningar krypteras och hanteras strikt konfidentiellt av ledningen. Vid akuta personskador, säkerställ först och främst medicinsk vård.
          </p>
        </div>

        <Tabs value={activeTab} className="space-y-4" onValueChange={setActiveTab}>
          <TabsList className="bg-muted/50 p-1">
            <TabsTrigger value="send" className="font-medium">
              {t('reporting.tabs.send')}
            </TabsTrigger>
            <TabsTrigger value="my_reports" className="font-medium">
              {t('reporting.tabs.my_reports')}
            </TabsTrigger>
          </TabsList>

        <TabsContent value="send" className="space-y-4">
          <div className="grid gap-4 md:grid-cols-1">
            <Card className="border-border/50 bg-card/50 backdrop-blur-sm">
              <CardHeader>
                <CardTitle>
                  {t('reporting.tabs.send')}
                </CardTitle>
                <CardDescription>
                  {t('reporting.form.description_help')}
                </CardDescription>
              </CardHeader>
              <CardContent>
                <ReportForm onSuccess={() => setActiveTab('my_reports')} />
              </CardContent>
            </Card>
          </div>
        </TabsContent>

        <TabsContent value="my_reports" className="space-y-4">
          <Card className="border-border/50 bg-card/50 backdrop-blur-sm">
            <CardHeader>
              <CardTitle>{t('reporting.tabs.my_reports')}</CardTitle>
              <CardDescription>
                {t('reporting.list.description')}
              </CardDescription>
            </CardHeader>
            <CardContent>
              <ReportList />
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>
      </div>
    </div>
  )
}
