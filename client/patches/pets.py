"""Small source transformations for the reference client's Pet Villa defects."""


def replace_once(source: str, old: str, new: str) -> str:
    if source.count(old) != 1:
        raise ValueError("Pet client patch expected exactly one match: " + old[:100])
    return source.replace(old, new, 1)


def patch_training(source: str) -> str:
    if "repairTimerLifecycle" in source:
        return source
    source = replace_once(source, "         this.slotHolder = param2;", "         this.stopTimer();\n         this.slotHolder = param2;")
    source = replace_once(source,
        "         this.trainTimer = null;\n         this.trainTimer.removeEventListener(TimerEvent.TIMER,this.onUpdateProgress);\n         this.trainTimer.removeEventListener(TimerEvent.TIMER_COMPLETE,this.onTimerComplete);",
        "         this.stopTimer();")
    source = replace_once(source,
        "         this.stopTimer();\n         GF.removeAllChild(this.slotHolder);",
        "         this.stopTimer();\n         this.slotHolder = null;")
    return replace_once(source, "      public function destroy() : *", "      private function repairTimerLifecycle() : void {}\n\n      public function destroy() : *")


def patch_villa(source: str) -> str:
    if "repairVillaLifecycle" in source:
        return source
    source = replace_once(source,
        "         this.panel.heyaDarenMC.visible = false;",
        "         this.panel.heyaDarenMC.stopAllMovieClips();\n         this.panel.heyaDarenMC.visible = false;")
    source = replace_once(source, "            this.slots = param1.slots;", "            this.clearAllTrainPetSlot();\n            this.slots = param1.slots;")
    source = source.replace("this.total_page = Math.ceil(this.pets.length / 6);", "this.total_page = Math.max(1,Math.ceil(this.pets.length / 6));")
    start = source.index("      public function findPet(")
    end = source.index("      public function displaySlots(", start)
    source = source[:start] + source[start:end].replace("this.pets", "this.all_pets") + source[end:]
    source = replace_once(source, "      public function displaySlots() : *\n      {", "      public function displaySlots() : *\n      {\n         this.clearAllTrainPetSlot();")
    # Always cancel queued icon loads before repopulating a reused holder.
    source = source.replace('GF.removeAllChild(this.panel["roomMC_" + _loc1_].iconHolder);', 'NinjaSage.cancelHolderLoads(this.panel["roomMC_" + _loc1_].iconHolder);\n               GF.removeAllChild(this.panel["roomMC_" + _loc1_].iconHolder);')
    source = replace_once(source,
        '         while(_loc11_ < 6)\n         {',
        '         while(_loc11_ < 6)\n         {\n            NinjaSage.cancelHolderLoads(this.panel["petMC_" + _loc11_].iconHolder);\n            GF.removeAllChild(this.panel["petMC_" + _loc11_].iconHolder);')
    source = replace_once(source, "            Character.account_tokens -= this.skipPrice;", "            Character.account_tokens = param1.hasOwnProperty(\"tokens\") ? param1.tokens : Character.account_tokens - this.skipPrice;")
    source = replace_once(source, "      public function trainPetResponse(param1:Object) : *\n      {", "      public function trainPetResponse(param1:Object) : *\n      {\n         if(param1.status == 1 && param1.hasOwnProperty(\"gold\")) { Character.character_gold = param1.gold; this.main.HUD.setBasicData(); }")
    source = replace_once(source, "         this.main.handleVillageHUDVisibility(true);", "         this.clearAllTrainPetSlot();\n         this.main.handleVillageHUDVisibility(true);")
    source = replace_once(source, "      private function getTrainData(", "      private function repairVillaLifecycle() : void {}\n\n      private function getTrainData(")
    return source


def patch_icon_loader(source: str) -> str:
    if "cancelHolderLoads" in source:
        return source
    source = replace_once(source,
        '            var _loc5_:* = param2 + "-" + NinjaSage.generateRandomString(4);',
        '            NinjaSage.cancelHolderLoads(param3);\n            var _loc5_:* = param2 + "-" + NinjaSage.generateRandomString(4);')
    source = replace_once(source,
        "            if(loader.hasItem(_loc1_.id,false))",
        "            if(loader && loader.hasItem(_loc1_.id,false) && loader.get(_loc1_.id).isLoaded && loader.get(_loc1_.id).content)")
    source = replace_once(source,
        '            if(!Character.play_items_animation)',
        '            if(!Character.play_items_animation || itemInfo.type == "PetStaticFullBody")')
    source = replace_once(source,
        "            param2.addChild(param1);",
        "            GF.removeAllChild(param2);\n            param2.addChild(param1);")
    source = replace_once(source,
        "         if(itemInfo.type != null)",
        '         if(itemInfo.type == "PetStaticFullBody" && item.content is MovieClip) { MovieClip(item.content).stopAllMovieClips(); }\n         if(itemInfo.type != null)')
    source = replace_once(source,
        "         loading = false;\n         loader.remove(itemInfo.id);",
        "         loading = false;\n         delete itemMaps[itemInfo.id];\n         loader.remove(itemInfo.id);")
    # Deleting map entries before removing loader items prevents reentrant
    # progress notifications from reattaching an old request to the same holder.
    helper = '''      public static function cancelHolderLoads(holder:*) : void
      {
         if(!loader) { return; }
         var stale:Array = [];
         var entry:*;
         for each(entry in itemMaps) {
            if(entry.holder == holder) { stale.push(entry.id); }
         }
         for each(var id:String in stale) {
            delete itemMaps[id];
            loader.remove(id);
         }
      }

'''
    return replace_once(source, "      public static function clearLoader() : *", helper + "      public static function clearLoader() : *")


PATCHES = {
    "Panels.PetTraining": patch_training,
    "Panels.PetVilla": patch_villa,
    "Managers.NinjaSage": patch_icon_loader,
}
VERIFICATIONS = {
    "Panels.PetTraining": ["repairTimerLifecycle", "this.stopTimer();"],
    "Panels.PetVilla": ["repairVillaLifecycle", "NinjaSage.cancelHolderLoads", "this.panel.heyaDarenMC.stopAllMovieClips();"],
    "Managers.NinjaSage": ["function cancelHolderLoads", ".isLoaded", "delete itemMaps[itemInfo.id]"],
}
