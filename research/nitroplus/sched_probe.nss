// The scheduler probe (ntrace.sh --probe, --dumps): the overlap cases of
// docs/tools.md, "Matching the original", with a memory dump before and after each.
// Run at the start of a new game (Django); labels: c<case>_<step>.
// c0: dja010_01:76-78 as it stands: a 16 s pan left running, then FadeFade.
CreateBGz("背景21",21, -560,-30, 700, "bg11100_0_断崖の上m");
Delete("__dump_c0_0");
Move("背景21",16000, @320,@0, null,false);
Delete("__dump_c0_1");
Fade("背景21", 0, 0, null, true);
Delete("__dump_c0_2");
Fade("背景21", 1000, 1000, null, true);
Delete("__dump_c0_3");
Wait(1000);
Delete("__dump_c0_4");
Delete("背景21");
// c1: an unwaited move goes on while other commands run (Wait, another picture's waited fade)
CreateColor("probeA",100,0,0,100,100,0xFF0000);
Move("probeA",4000, @400,@0, null,false);
Delete("__dump_c1_0");
CreateColor("probeB",101,0,200,100,100,0x00FF00);
Fade("probeB", 1000, 500, null, true);
Delete("__dump_c1_1");
// c2: a 0 ms action on one in flight (move, then fade)
Move("probeA",0, @0,@100, null,false);
Delete("__dump_c2_0");
Wait(500);
Delete("__dump_c2_1");
Fade("probeB", 4000, 0, null, false);
Wait(500);
Delete("__dump_c2_2");
Fade("probeB", 0, 1000, null, false);
Delete("__dump_c2_3");
Wait(500);
Delete("__dump_c2_4");
// c3: a wait on one in flight (a waited move over an unwaited one; WaitMove)
Move("probeA",4000, @400,@0, null,false);
Wait(500);
Delete("__dump_c3_0");
Move("probeA",500, @0,@100, null,true);
Delete("__dump_c3_1");
Move("probeA",2000, @200,@0, null,false);
WaitMove("probeA");
Delete("__dump_c3_2");
// c4: delete or replace while moving
Move("probeA",4000, @400,@0, null,false);
Wait(500);
Delete("__dump_c4_0");
CreateColor("probeA",100,0,300,100,100,0x0000FF);
Delete("__dump_c4_1");
Wait(500);
Delete("__dump_c4_2");
Move("probeB",4000, @400,@0, null,false);
Wait(500);
Delete("probeB");
CreateColor("probeB",101,0,400,100,100,0x00FF00);
Delete("__dump_c4_3");
Wait(500);
Delete("__dump_c4_4");
Delete("probe*");
