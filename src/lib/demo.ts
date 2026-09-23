/**
 * Demo mode — runs the entire app without a Supabase backend.
 * Enabled via VITE_DEMO_MODE=true. All data lives in memory (module level),
 * so reads and writes work for the session but reset on reload.
 */

export const isDemoMode = (): boolean => import.meta.env.VITE_DEMO_MODE === 'true'

export const DEMO_WORKSPACE_ID = 'demo-workspace'
export const DEMO_USER_ID = 'u-admin'

const day = 24 * 60 * 60 * 1000
const hour = 60 * 60 * 1000
const now = Date.now()

const iso = (t: number) => new Date(t).toISOString()
const at = (dayOffset: number, h: number, m = 0) => {
  const d = new Date(now + dayOffset * day)
  d.setHours(h, m, 0, 0)
  return d.getTime()
}

export type DemoRow = Record<string, any>

function seed() {
  const users: DemoRow[] = [
    {
      id: DEMO_USER_ID,
      email: 'admin@yntra.se',
      full_name: 'Anna Andersson',
      role: 'admin',
      workspace_id: DEMO_WORKSPACE_ID,
      phone: '070-123 45 67',
      location: 'Stockholm',
      client_id: null,
      privacy_settings: {},
      preferences: { theme: 'system', calendar_density: 'relaxed', font_scale: 1.0 },
      created_at: iso(now - 90 * day),
    },
    {
      id: 'u-erik',
      email: 'erik@yntra.se',
      full_name: 'Erik Eriksson',
      role: 'assistant',
      workspace_id: DEMO_WORKSPACE_ID,
      phone: '070-234 56 78',
      location: 'Stockholm',
      client_id: null,
      privacy_settings: {},
      preferences: {},
      created_at: iso(now - 85 * day),
    },
    {
      id: 'u-sara',
      email: 'sara@yntra.se',
      full_name: 'Sara Svensson',
      role: 'assistant',
      workspace_id: DEMO_WORKSPACE_ID,
      phone: '070-345 67 89',
      location: 'Uppsala',
      client_id: null,
      privacy_settings: {},
      preferences: {},
      created_at: iso(now - 80 * day),
    },
    {
      id: 'u-johan',
      email: 'johan@yntra.se',
      full_name: 'Johan Johansson',
      role: 'user',
      workspace_id: DEMO_WORKSPACE_ID,
      phone: '070-456 78 90',
      location: 'Stockholm',
      client_id: null,
      privacy_settings: {},
      preferences: {},
      created_at: iso(now - 70 * day),
    },
    {
      id: 'u-maria',
      email: 'maria@yntra.se',
      full_name: 'Maria Nilsson',
      role: 'assistant',
      workspace_id: DEMO_WORKSPACE_ID,
      phone: '070-567 89 01',
      location: 'Sollentuna',
      client_id: null,
      privacy_settings: {},
      preferences: {},
      created_at: iso(now - 60 * day),
    },
  ]

  const workspaces: DemoRow[] = [
    {
      id: DEMO_WORKSPACE_ID,
      name: 'Yntra Demo AB',
      logo_url: null,
      brand_color: '#3b82f6',
      settings: {
        timezone: 'Europe/Stockholm',
        week_start: 1,
        language: 'sv',
        business_hours: { start: 0, end: 23 },
      },
      modules_active: {
        school: false,
        assistance: true,
        messaging: true,
        scheduling: true,
        notes: true,
        time: true,
        directory: true,
        reporting: true,
      },
      block_settings: {},
      created_at: iso(now - 120 * day),
    },
  ]

  const teams: DemoRow[] = [
    { id: 't-norr', name: 'Team Norr', workspace_id: DEMO_WORKSPACE_ID, created_at: iso(now - 100 * day) },
    { id: 't-soder', name: 'Team Söder', workspace_id: DEMO_WORKSPACE_ID, created_at: iso(now - 100 * day) },
  ]

  const team_members: DemoRow[] = [
    { id: 'tm-1', team_id: 't-norr', user_id: 'u-erik', workspace_id: DEMO_WORKSPACE_ID },
    { id: 'tm-2', team_id: 't-norr', user_id: 'u-sara', workspace_id: DEMO_WORKSPACE_ID },
    { id: 'tm-3', team_id: 't-soder', user_id: 'u-johan', workspace_id: DEMO_WORKSPACE_ID },
    { id: 'tm-4', team_id: 't-soder', user_id: 'u-maria', workspace_id: DEMO_WORKSPACE_ID },
    { id: 'tm-5', team_id: 't-norr', user_id: DEMO_USER_ID, workspace_id: DEMO_WORKSPACE_ID },
  ]

  const workspace_roles: DemoRow[] = [
    { id: 'wr-1', workspace_id: DEMO_WORKSPACE_ID, user_id: DEMO_USER_ID, role: 'admin' },
    { id: 'wr-2', workspace_id: DEMO_WORKSPACE_ID, user_id: 'u-erik', role: 'assistant' },
  ]

  const events: DemoRow[] = [
    {
      id: 'e-1',
      workspace_id: DEMO_WORKSPACE_ID,
      user_id: 'u-erik',
      team_id: 't-norr',
      assignee_id: 'u-erik',
      title: 'Assistance hos Lindgren',
      start_time: iso(at(0, 9)),
      end_time: iso(at(0, 9) + 3 * hour),
      metadata: { category: 'assistance', description: 'Medicinutdelning och promenad.', location: 'Kungsgatan 12', isAllDay: false, attendees: ['u-erik'] },
      created_at: iso(now - 5 * day),
    },
    {
      id: 'e-2',
      workspace_id: DEMO_WORKSPACE_ID,
      user_id: 'u-sara',
      team_id: 't-norr',
      assignee_id: 'u-sara',
      title: 'Läkemedelsgenomgång',
      start_time: iso(at(0, 13)),
      end_time: iso(at(0, 9 + 4) + 0 * hour),
      metadata: { category: 'medical', description: 'Veckovis genomgång med sjuksköterska.', location: 'Distriktssköterskemottagningen', isAllDay: false, attendees: ['u-sara', DEMO_USER_ID] },
      created_at: iso(now - 4 * day),
    },
    {
      id: 'e-3',
      workspace_id: DEMO_WORKSPACE_ID,
      user_id: DEMO_USER_ID,
      team_id: 't-soder',
      assignee_id: 'u-maria',
      title: 'Introduktion ny assistent',
      start_time: iso(at(1, 10)),
      end_time: iso(at(1, 10) + 2 * hour),
      metadata: { category: 'introduction', description: 'Genomgång av rutiner och journalföring.', location: 'Kontoret', isAllDay: false, attendees: ['u-maria', DEMO_USER_ID] },
      created_at: iso(now - 3 * day),
    },
    {
      id: 'e-4',
      workspace_id: DEMO_WORKSPACE_ID,
      user_id: 'u-johan',
      team_id: 't-soder',
      assignee_id: 'u-johan',
      title: 'Team-möte Söder',
      start_time: iso(at(2, 14)),
      end_time: iso(at(2, 14) + 1 * hour),
      metadata: { category: 'meeting', description: 'Veckomöte med teamet.', location: 'Teams', isAllDay: false, attendees: ['u-johan', 'u-maria'] },
      created_at: iso(now - 6 * day),
    },
    {
      id: 'e-5',
      workspace_id: DEMO_WORKSPACE_ID,
      user_id: 'u-erik',
      team_id: 't-norr',
      assignee_id: 'u-erik',
      title: 'Avlösning hos Bergström',
      start_time: iso(at(-1, 15)),
      end_time: iso(at(-1, 15) + 2 * hour),
      metadata: { category: 'schedule', description: '', location: 'Bergströms väg 4', isAllDay: false, attendees: ['u-erik'] },
      created_at: iso(now - 7 * day),
    },
    {
      id: 'e-6',
      workspace_id: DEMO_WORKSPACE_ID,
      user_id: DEMO_USER_ID,
      team_id: null,
      assignee_id: DEMO_USER_ID,
      title: 'Semesteransökan – deadline',
      start_time: iso(at(4, 12)),
      end_time: iso(at(4, 13)),
      metadata: { category: 'administrative_hours', description: 'Sista dag att lämna in sommaransökan.', location: '', isAllDay: true, attendees: [] },
      created_at: iso(now - 2 * day),
    },
    {
      id: 'e-7',
      workspace_id: DEMO_WORKSPACE_ID,
      user_id: DEMO_USER_ID,
      team_id: 't-norr',
      assignee_id: DEMO_USER_ID,
      title: 'Förmiddagspass hos Greta Lindgren',
      start_time: iso(at(-1, 8)),
      end_time: iso(at(-1, 16)),
      metadata: { category: 'schedule', description: 'Assistanstimmar och lunchstöd', location: 'Lindgren, Norr', isAllDay: false, attendees: [DEMO_USER_ID] },
      created_at: iso(now - 2 * day),
    },
    {
      id: 'e-8',
      workspace_id: DEMO_WORKSPACE_ID,
      user_id: DEMO_USER_ID,
      team_id: 't-norr',
      assignee_id: DEMO_USER_ID,
      title: 'Kvällspass och medicinöverlämning',
      start_time: iso(at(-2, 16)),
      end_time: iso(at(-2, 21)),
      metadata: { category: 'schedule', description: 'Kvällsrutin och dokumentation', location: 'Lindgren, Norr', isAllDay: false, attendees: [DEMO_USER_ID] },
      created_at: iso(now - 3 * day),
    },
    {
      id: 'e-9',
      workspace_id: DEMO_WORKSPACE_ID,
      user_id: DEMO_USER_ID,
      team_id: 't-soder',
      assignee_id: DEMO_USER_ID,
      title: 'Helgpass hos Torsten Holm',
      start_time: iso(at(-3, 9)),
      end_time: iso(at(-3, 17)),
      metadata: { category: 'schedule', description: 'Dagaktiviteter och promenad', location: 'Södermalm', isAllDay: false, attendees: [DEMO_USER_ID] },
      created_at: iso(now - 4 * day),
    },
    {
      id: 'e-10',
      workspace_id: DEMO_WORKSPACE_ID,
      user_id: 'u-sara',
      team_id: 't-norr',
      assignee_id: 'u-sara',
      title: 'Extrapass Norr',
      start_time: iso(at(-2, 8)),
      end_time: iso(at(-2, 16)),
      metadata: { category: 'schedule', description: 'Täckte upp för kollega', location: 'Norr', isAllDay: false, attendees: ['u-sara'] },
      created_at: iso(now - 3 * day),
    },
  ]

  const messages: DemoRow[] = [
    {
      id: 'm-1', workspace_id: DEMO_WORKSPACE_ID, sender_id: 'u-erik', receiver_id: DEMO_USER_ID,
      target_team_id: null, subject: 'Bil havererade på väg till Lindgren',
      body: 'Hej! Bilen vägrar starta efter lunchbesöket. Jag har bokat assistans och är tillbaka på kontoret vid 15-tiden. /Erik',
      is_read: false, created_at: iso(now - 2 * hour),
    },
    {
      id: 'm-2', workspace_id: DEMO_WORKSPACE_ID, sender_id: 'u-sara', receiver_id: DEMO_USER_ID,
      target_team_id: null, subject: 'Recept förnyat',
      body: 'Hej Anna, receptet på morfin för Lindgren är förnyat och ligger redo på apoteket från imorgon. Med vänliga hälsningar, Sara',
      is_read: false, created_at: iso(now - 6 * hour),
    },
    {
      id: 'm-3', workspace_id: DEMO_WORKSPACE_ID, sender_id: DEMO_USER_ID, receiver_id: 'u-erik',
      target_team_id: 't-norr', subject: 'Schema vecka 38',
      body: 'Hej teamet! Schemat för nästa vecka är nu publicerat i kalendern. Kolla era pass och hör av er vid frågor.',
      is_read: true, created_at: iso(now - 1 * day),
    },
    {
      id: 'm-4', workspace_id: DEMO_WORKSPACE_ID, sender_id: 'u-maria', receiver_id: DEMO_USER_ID,
      target_team_id: null, subject: 'Nyckelkort till Söder',
      body: 'Johan behöver ett nytt nyckelkort till Brunnsgatan 8. Kan du beställa ett?',
      is_read: true, created_at: iso(now - 2 * day),
    },
    {
      id: 'm-5', workspace_id: DEMO_WORKSPACE_ID, sender_id: 'u-johan', receiver_id: DEMO_USER_ID,
      target_team_id: null, subject: 'Tidrapporter godkända',
      body: 'Tack! Alla tidrapporter för förra veckan är nu godkända och klara.',
      is_read: true, created_at: iso(now - 3 * day),
    },
    {
      id: 'm-6', workspace_id: DEMO_WORKSPACE_ID, sender_id: 'u-erik', receiver_id: DEMO_USER_ID,
      target_team_id: null, subject: 'VAB imorgon?',
      body: 'Min dotter är sjuk, kan jag ta VAB imorgon förmiddag och jobba ikväll istället?',
      is_read: false, created_at: iso(now - 30 * 60 * 1000),
    },
  ]

  const notes: DemoRow[] = [
    {
      id: 'n-1', team_id: 't-norr', workspace_id: DEMO_WORKSPACE_ID, author_id: 'u-sara',
      subject: 'Insatsplan Lindgren uppdaterad',
      content: 'Insatsplanen för Greta Lindgren är uppdaterad efter tisdagens biståndsbeslut. Nytt fokus på kvällsinsatser och medicinhantering. Alla i teamet behöver läsa igenom den nya planen innan fredag.',
      created_at: iso(now - 3 * hour), edit_history: null,
    },
    {
      id: 'n-2', team_id: 't-norr', workspace_id: DEMO_WORKSPACE_ID, author_id: 'u-erik',
      subject: 'Hiss underhåll Bergström',
      content: 'Fastighetsbolaget kommer och servar hissen på Bergströms väg 4 på torsdag 09:00–11:00. Planera om Morgonpasset att använda trapporna eller hjälpa via hissen i grannhuset.',
      created_at: iso(now - 1 * day), edit_history: null,
    },
    {
      id: 'n-3', team_id: 't-soder', workspace_id: DEMO_WORKSPACE_ID, author_id: 'u-maria',
      subject: 'Ny kund: Holm, Södermalm',
      content: 'Vi tar över insatserna för Torsten Holm från den 1:a. Boka in erfarenhetsöverföring med tidigare utförare under nästa vecka. Journal och kontaktuppgifter ligger i brukarportalen.',
      created_at: iso(now - 2 * day), edit_history: null,
    },
    {
      id: 'n-4', team_id: 't-norr', workspace_id: DEMO_WORKSPACE_ID, author_id: DEMO_USER_ID,
      subject: 'Sommarschema',
      content: 'Påminnelse: önskemål för sommarschemat ska vara inne till fredag. Lägg era önskemål direkt i kalendern eller maila mig.',
      created_at: iso(now - 4 * day), edit_history: null,
    },
    {
      id: 'n-5', team_id: 't-soder', workspace_id: DEMO_WORKSPACE_ID, author_id: 'u-johan',
      subject: 'Brandövning kontoret',
      content: 'Brandövning tisdag kl 12:00. Samling vid entrén. Tar cirka 15 minuter.',
      created_at: iso(now - 5 * day), edit_history: null,
    },
  ]

  const reports: DemoRow[] = [
    {
      id: 'r-1', workspace_id: DEMO_WORKSPACE_ID, user_id: 'u-erik', type: 'deviation',
      is_anonymous: false, status: 'pending',
      content: { subject: 'Missad medicinering', description: 'Lunchtabletten glömdes vid 12-passet den 14/9. Anhörig informerad enligt rutin.', date_of_incident: iso(at(-3, 12)) },
      created_at: iso(now - 2 * day),
    },
    {
      id: 'r-2', workspace_id: DEMO_WORKSPACE_ID, user_id: 'u-sara', type: 'deviation',
      is_anonymous: false, status: 'pending',
      content: { subject: 'Fel dos i dosetten', description: 'Dosetten för levererad vecka hade två tabletter för myckig. Ny dosett levererad samma dag.', date_of_incident: iso(at(-5, 9)) },
      created_at: iso(now - 4 * day),
    },
    {
      id: 'r-3', workspace_id: DEMO_WORKSPACE_ID, user_id: 'u-johan', type: 'feedback',
      is_anonymous: false, status: 'approved',
      content: { subject: 'Förslag: digitalt nyckelskåp', description: 'Föreslår digitalt nyckelskåp vid Brunnsgatan för att slippa dubbla uppsättningar.', date_of_incident: iso(at(-10, 10)) },
      created_at: iso(now - 9 * day),
    },
    {
      id: 'r-4', workspace_id: DEMO_WORKSPACE_ID, user_id: 'u-maria', type: 'sick',
      is_anonymous: false, status: 'approved',
      content: { subject: 'Sjukdomsfall – förkylning', description: 'Feber och hosta, kan inte arbeta måndag–tisdag.', date_of_incident: iso(at(-7, 8)) },
      created_at: iso(now - 8 * day),
    },
  ]

  const clients: DemoRow[] = [
    { id: 'c-1', first_name: 'Greta', last_name: 'Lindgren', personal_number: '19420315-1234', care_level: '2', workspace_id: DEMO_WORKSPACE_ID },
    { id: 'c-2', first_name: 'Sven', last_name: 'Bergström', personal_number: '19371102-5678', care_level: '3', workspace_id: DEMO_WORKSPACE_ID },
    { id: 'c-3', first_name: 'Torsten', last_name: 'Holm', personal_number: '19500520-9012', care_level: '1', workspace_id: DEMO_WORKSPACE_ID },
    { id: 'c-4', first_name: 'Astrid', last_name: 'Ek', personal_number: '19480830-3456', care_level: '2', workspace_id: DEMO_WORKSPACE_ID },
  ]

  const client_medications: DemoRow[] = [
    { id: 'med-1', client_id: 'c-1', name: 'Metoprolol', dosage: '50 mg', frequency: '2 gånger dagligen', instructions: 'Ta med mat på morgonen och kvällen.' },
    { id: 'med-2', client_id: 'c-1', name: 'Morfin', dosage: '10 mg', frequency: 'Vid behov, max 3 gånger dagligen', instructions: 'Observera andning vid intag.' },
    { id: 'med-3', client_id: 'c-2', name: 'Waran', dosage: '5 mg', frequency: '1 tablett dagligen', instructions: 'INR-prov varannan vecka.' },
    { id: 'med-4', client_id: 'c-3', name: 'Alvedon', dosage: '500 mg', frequency: 'Vid behov', instructions: 'Max 4 gånger dagligen.' },
  ]

  const client_journals: DemoRow[] = [
    { id: 'j-1', client_id: 'c-1', author_id: 'u-sara', content: 'Greta är piggare idag. Promenad i 20 minuter på förmiddagen. Åt hela lunchen.', created_at: iso(at(0, 13)) },
    { id: 'j-2', client_id: 'c-1', author_id: 'u-erik', content: 'Kvällsinsatsen gick bra. Greta berättade om barnbarnen och sommarstugan.', created_at: iso(at(-1, 20)) },
    { id: 'j-3', client_id: 'c-2', author_id: 'u-maria', content: 'Sven har svårt att komma upp på morgonen denna vecka. Anpassa insatserna.', created_at: iso(at(-1, 9)) },
  ]

  const time_reports: DemoRow[] = [
    { id: 'tr-1', workspace_id: DEMO_WORKSPACE_ID, user_id: 'u-erik', team_id: 't-norr', shift_id: null, date: iso(at(-4, 8)), start_time: '08:00', end_time: '16:00', hours: 8, breaks: 30, status: 'approved', note: 'Ordinarie dagpass', category: 'assistance', created_at: iso(at(-4, 16)) },
    { id: 'tr-2', workspace_id: DEMO_WORKSPACE_ID, user_id: 'u-sara', team_id: 't-norr', shift_id: null, date: iso(at(-3, 9)), start_time: '09:00', end_time: '15:00', hours: 6, breaks: 45, status: 'approved', note: 'Assistanstimmar', category: 'assistance', created_at: iso(at(-3, 15)) },
    { id: 'tr-3', workspace_id: DEMO_WORKSPACE_ID, user_id: 'u-johan', team_id: 't-soder', shift_id: null, date: iso(at(-1, 8)), start_time: '08:00', end_time: '16:00', hours: 8, breaks: 0, status: 'pending_attest', note: 'Beredskapspass', category: 'on_call', created_at: iso(at(-1, 16)) },
    { id: 'tr-4', workspace_id: DEMO_WORKSPACE_ID, user_id: 'u-maria', team_id: 't-soder', shift_id: null, date: iso(at(-2, 7)), start_time: '07:00', end_time: '15:00', hours: 8, breaks: 30, status: 'pending_attest', note: 'Schemapass söder', category: 'schedule', created_at: iso(at(-2, 15)) },
    { id: 'tr-5', workspace_id: DEMO_WORKSPACE_ID, user_id: DEMO_USER_ID, team_id: 't-norr', shift_id: null, date: iso(at(-5, 8)), start_time: '08:00', end_time: '16:00', hours: 8, breaks: 30, status: 'approved', note: 'Helgdagsassistans', category: 'schedule', created_at: iso(at(-5, 16)) },
  ]

  return {
    users,
    workspaces,
    teams,
    team_members,
    workspace_roles,
    events,
    messages,
    notes,
    reports,
    clients,
    client_medications,
    client_journals,
    time_reports,
    blocks: [],
    logos: [],
  }
}

export const demoDB: Record<string, DemoRow[]> = seed()

export const demoAuthUser = {
  id: DEMO_USER_ID,
  email: 'admin@yntra.se',
  user_metadata: { role: 'admin', full_name: 'Anna Andersson' },
  last_sign_in_at: iso(now),
}
