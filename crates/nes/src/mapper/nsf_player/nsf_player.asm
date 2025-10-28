; cl65 --config nsf_player.cfg --cpu 6502 nsf_player.asm -o nsf_player.bin

PpuCtrl = $2000
PpuMask = $2001
PpuStatus = $2002
PpuScroll = $2006
PpuAddr = $2006
PpuData = $2007
JoyPad1 = $4016

JoyPadLeft = %00000010
JoyPadRight = %00000001

NsfPlayTimer = $5300
NsfRegion = $5301
NsfInitBanks = $5302
NsfCurrentSong = $5303
NsfNextSong = $5304
NsfPrevSong = $5305
NsfInitSub = $5310
NsfPlaySub = $5320

.zeropage

.bss
buttons: .res 1
buttons_reset: .res 1

.code
InterruptVector:
    jmp InitSystem
ResetVector:
    jmp InitSystem
NmiVector:
    jmp InitSystemNmi

InitSystemNmi:
    lda #$00
    sta PpuCtrl
InitSystem:
    bit PpuStatus
@PpuWait1:
    bit PpuStatus
    bpl @PpuWait1
InitSong:
@PpuWait2:
    bit PpuStatus
    bpl @PpuWait2
    sei
    lda #$00
    sta PpuCtrl
    lda #$0e
    sta PpuMask
    lda #$3f
    sta PpuAddr
    lda #$00
    sta PpuAddr
    ldy #$08
@SetPalette:
    lda #$0f
    sta PpuData
    lda #$00
    sta PpuData
    lda #$10
    sta PpuData
    lda #$30
    sta PpuData
    dey
    bne @SetPalette
    lda #$00
    sta PpuScroll
    sta PpuScroll
    
    ldx #$00
@ClearRam:
    sta $000,x
    sta $100,x
    sta $200,x
    sta $300,x
    sta $400,x
    sta $500,x
    sta $600,x
    sta $700,x
    inx
    bne @ClearRam
    
    lda #$00
    ldx #$15
@LoopX:
    sta $4000,x
    dex
    bne @LoopX
    sta $4000
    lda $0f
    sta $4015
    lda $40
    sta $4017
    sta NsfInitBanks
    lda NsfCurrentSong
    ldx NsfRegion
    ldy #$00
    jsr NsfInitSub

PlayWait:
    lda NsfPlayTimer
    beq PlayWait
    jsr NsfPlaySub
    jsr ReadJoySafe
    lda buttons
    and #$ff
    beq ResetButtons
    lda buttons_reset
    beq PlayWait
    lda buttons
    and #JoyPadLeft
    bne PrevSong
    lda buttons
    and #JoyPadRight
    bne NextSong
    jmp PlayWait

PrevSong:
    lda #$00
    sta buttons_reset
    lda NsfPrevSong
    jmp InitSong

NextSong:
    lda #$00
    sta buttons_reset
    lda NsfNextSong
    jmp InitSong
    
ResetButtons:
    lda #$01
    sta buttons_reset
    jmp PlayWait

ReadJoy:
    lda #$01
    sta JoyPad1
    sta buttons
    lsr a
    sta JoyPad1
@Loop:
    lda JoyPad1
    and #%00000011
    cmp #$01
    rol buttons
    bcc @Loop
    rts

ReadJoySafe:
    jsr ReadJoy
@ReRead:
    lda buttons
    pha
    jsr ReadJoy
    pla
    cmp buttons
    bne @ReRead
    rts
