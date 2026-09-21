import { serve } from 'https://deno.land/std@0.168.0/http/server.ts'
import { createClient } from 'https://esm.sh/@supabase/supabase-js@2'

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const RESEND_API_KEY = (Deno as any).env.get('RESEND_API_KEY')
// eslint-disable-next-line @typescript-eslint/no-explicit-any
const SUPABASE_URL = (Deno as any).env.get('SUPABASE_INTERNAL_URL') // Or SUPABASE_URL
// eslint-disable-next-line @typescript-eslint/no-explicit-any
const SUPABASE_SERVICE_ROLE_KEY = (Deno as any).env.get('SUPABASE_SERVICE_ROLE_KEY')

const corsHeaders = {
  'Access-Control-Allow-Origin': '*',
  'Access-Control-Allow-Headers': 'authorization, x-client-info, apikey, content-type',
}

serve(async (req: Request) => {
  if (req.method === 'OPTIONS') {
    return new Response('ok', { headers: corsHeaders })
  }

  try {
    const supabase = createClient(SUPABASE_URL!, SUPABASE_SERVICE_ROLE_KEY!)

    const payload = await req.json()
    const { table, type, record } = payload

    if (type !== 'INSERT' && type !== 'UPDATE') {
      return new Response(JSON.stringify({ message: 'Ignore' }), {
        headers: { ...corsHeaders, 'Content-Type': 'application/json' },
      })
    }

    const { data: workspace } = await supabase
      .from('workspaces')
      .select('settings')
      .eq('id', record.workspace_id)
      .single()

    const lang = (workspace?.settings?.language === 'en' ? 'en' : 'sv') as 'sv' | 'en'

    const translations = {
      sv: {
        new_message: 'Nytt meddelande',
        no_subject: 'Ingen rubrik',
        alert_body: 'Du har fått ett nytt meddelande i Inkorg från {{sender}}.',
        full_body: 'Nytt meddelande från {{sender}}:\n\n{{body}}',
        team_message: 'Team-meddelande',
        team_alert_body: 'Ditt team har fått ett nytt meddelande i Inkorg.',
        team_full_body: 'Nytt team-meddelande:\n\n{{body}}',
        report_approved_subject: 'Din tidsrapport har blivit godkänd',
        report_approved_body:
          'Hej!\n\nDin tidsrapport för {{date}} ({{hours}}h) har blivit godkänd av din chef.',
        system: 'System',
      },
      en: {
        new_message: 'New message',
        no_subject: 'No subject',
        alert_body: 'You have received a new message in Inbox from {{sender}}.',
        full_body: 'New message from {{sender}}:\n\n{{body}}',
        team_message: 'Team message',
        team_alert_body: 'Your team has received a new message in Inbox.',
        team_full_body: 'New team message:\n\n{{body}}',
        report_approved_subject: 'Your time report has been approved',
        report_approved_body:
          'Hi!\n\nYour time report for {{date}} ({{hours}}h) has been approved by your manager.',
        system: 'System',
      },
    }

    const t = translations[lang]

    const notifications: { email: string; subject: string; body: string }[] = []

    if (table === 'messages' && type === 'INSERT') {
      const msg = record
      if (msg.receiver_id) {
        // Individual message
        const { data: user } = await supabase
          .from('users')
          .select('email, full_name, notifications_on, notification_type')
          .eq('id', msg.receiver_id)
          .single()
        if (user?.notifications_on) {
          notifications.push({
            email: user.email,
            subject: `${t.new_message}: ${msg.subject || t.no_subject}`,
            body:
              user.notification_type === 'alert_only'
                ? t.alert_body.replace('{{sender}}', msg.sender_id || t.system)
                : t.full_body.replace('{{sender}}', msg.sender_id || t.system).replace('{{body}}', msg.body),
          })
        }
      } else if (msg.target_team_id) {
        // Team message
        const { data: members } = await supabase
          .from('team_members')
          .select('user_id')
          .eq('team_id', msg.target_team_id)
        if (members) {
          const userIds = members.map((m: { user_id: string }) => m.user_id)
          const { data: users } = await supabase
            .from('users')
            .select('email, notifications_on, notification_type')
            .in('id', userIds)
          users?.forEach(
            (u: { email: string; notifications_on: boolean; notification_type: string }) => {
              if (u.notifications_on) {
                notifications.push({
                  email: u.email,
                  subject: `${t.team_message}: ${msg.subject || t.no_subject}`,
                  body:
                    u.notification_type === 'alert_only'
                      ? t.team_alert_body
                      : t.team_full_body.replace('{{body}}', msg.body),
                })
              }
            },
          )
        }
      }
    } else if (table === 'time_reports' && type === 'UPDATE') {
      const report = record
      if (report.status === 'approved') {
        const { data: user } = await supabase
          .from('users')
          .select('email, notifications_on, notification_type')
          .eq('id', report.user_id)
          .single()
        if (user?.notifications_on) {
          notifications.push({
            email: user.email,
            subject: t.report_approved_subject,
            body: t.report_approved_body
              .replace('{{date}}', report.date)
              .replace('{{hours}}', report.hours.toString()),
          })
        }
      }
    }

    for (const note of notifications) {
      await fetch('https://api.resend.com/emails', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${RESEND_API_KEY}`,
        },
        body: JSON.stringify({
          from: 'Yntra <notifications@yntra.se>',
          to: [note.email],
          subject: note.subject,
          text: note.body,
        }),
      })
    }

    return new Response(JSON.stringify({ success: true, count: notifications.length }), {
      headers: { ...corsHeaders, 'Content-Type': 'application/json' },
      status: 200,
    })
  } catch (error) {
    const message = error instanceof Error ? error.message : 'Unknown error'
    return new Response(JSON.stringify({ error: message }), {
      headers: { ...corsHeaders, 'Content-Type': 'application/json' },
      status: 400,
    })
  }
})
